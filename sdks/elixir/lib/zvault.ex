defmodule ZVault do
  @moduledoc """
  ZVault SDK for Elixir.

  Fetches secrets from ZVault Cloud at runtime. Caches in-memory via ETS,
  auto-refreshes on TTL, and gracefully degrades if the API is unreachable.

  ## Installation

  Add to `mix.exs`:

      def deps do
        [{:zvault, "~> 0.1.0"}]
      end

  ## Usage

      # Start the client (usually in Application supervisor)
      {:ok, _pid} = ZVault.start_link(token: System.get_env("ZVAULT_TOKEN"))

      # Fetch all secrets
      {:ok, secrets} = ZVault.get_all("production")
      db_url = Map.get(secrets, "DATABASE_URL")

      # Fetch single secret
      {:ok, value} = ZVault.get("STRIPE_KEY", "production")

      # List keys
      {:ok, keys} = ZVault.list_keys("production")

      # Health check
      {:ok, health} = ZVault.healthy()
  """

  use GenServer

  @default_base_url "https://api.zvault.cloud"
  @default_cache_ttl 300_000
  @default_timeout 10_000
  @default_max_retries 3
  @retry_base_delay 500

  # --- Public API ---

  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc "Fetch all secrets for an environment. Returns `{:ok, %{key => value}}`."
  def get_all(env \\ nil) do
    GenServer.call(__MODULE__, {:get_all, env}, 30_000)
  end

  @doc "Fetch a single secret by key. Returns `{:ok, value}` or `{:error, :not_found}`."
  def get(key, env \\ nil) do
    GenServer.call(__MODULE__, {:get, key, env}, 15_000)
  end

  @doc "Set a secret value. Returns `{:ok, secret}`."
  def set(key, value, env \\ nil, comment \\ "") do
    GenServer.call(__MODULE__, {:set, key, value, env, comment}, 15_000)
  end

  @doc "Delete a secret. Returns `:ok`."
  def delete(key, env \\ nil) do
    GenServer.call(__MODULE__, {:delete, key, env}, 15_000)
  end

  @doc "List secret keys (no values). Returns `{:ok, [%{key: key, ...}]}`."
  def list_keys(env \\ nil) do
    GenServer.call(__MODULE__, {:list_keys, env}, 15_000)
  end

  @doc "Check API health. Returns `{:ok, %{ok: bool, latency_ms: int}}`."
  def healthy do
    GenServer.call(__MODULE__, :healthy, 15_000)
  end

  @doc "Inject all secrets into System environment."
  def inject_into_env(env \\ nil) do
    case get_all(env) do
      {:ok, secrets} ->
        count =
          Enum.reduce(secrets, 0, fn {k, v}, acc ->
            System.put_env(k, v)
            acc + 1
          end)

        {:ok, count}

      error ->
        error
    end
  end

  # --- GenServer callbacks ---

  @impl true
  def init(opts) do
    token = opts[:token] || System.get_env("ZVAULT_TOKEN") || ""
    base_url = opts[:base_url] || System.get_env("ZVAULT_URL") || @default_base_url
    org_id = opts[:org_id] || System.get_env("ZVAULT_ORG_ID") || ""
    project_id = opts[:project_id] || System.get_env("ZVAULT_PROJECT_ID") || ""
    default_env = opts[:default_env] || System.get_env("ZVAULT_ENV") || "development"
    cache_ttl = opts[:cache_ttl] || @default_cache_ttl
    timeout = opts[:timeout] || @default_timeout
    max_retries = opts[:max_retries] || @default_max_retries
    debug = opts[:debug] || false

    if token == "" do
      {:stop, "Missing token. Set ZVAULT_TOKEN or pass token: option."}
    else
      cache_table = :ets.new(:zvault_cache, [:set, :private])

      state = %{
        token: token,
        base_url: String.trim_trailing(base_url, "/"),
        org_id: org_id,
        project_id: project_id,
        default_env: default_env,
        cache_ttl: cache_ttl,
        timeout: timeout,
        max_retries: max_retries,
        debug: debug,
        cache: cache_table,
        last_refresh: nil
      }

      {:ok, state}
    end
  end

  @impl true
  def handle_call({:get_all, env}, _from, state) do
    env = env || state.default_env

    case fetch_all_secrets(state, env) do
      {:ok, secrets} ->
        cache_put_all(state.cache, env, secrets, state.cache_ttl)
        state = %{state | last_refresh: DateTime.utc_now()}
        {:reply, {:ok, secrets}, state}

      {:error, _reason} = err ->
        cached = cache_get_all(state.cache, env)

        if map_size(cached) > 0 do
          log(state, "API unreachable, serving #{map_size(cached)} cached secrets")
          {:reply, {:ok, cached}, state}
        else
          {:reply, err, state}
        end
    end
  end

  @impl true
  def handle_call({:get, key, env}, _from, state) do
    env = env || state.default_env

    case cache_get(state.cache, env, key) do
      {:ok, value} ->
        {:reply, {:ok, value}, state}

      :miss ->
        case fetch_secret(state, env, key) do
          {:ok, value} ->
            cache_put(state.cache, env, key, value, state.cache_ttl)
            {:reply, {:ok, value}, state}

          {:error, _} = err ->
            {:reply, err, state}
        end
    end
  end

  @impl true
  def handle_call({:set, key, value, env, comment}, _from, state) do
    env = env || state.default_env
    path = secret_path(state, env, key)
    body = Jason.encode!(%{value: value, comment: comment})

    case request(state, "PUT", path, body) do
      {:ok, %{"secret" => secret}} ->
        cache_put(state.cache, env, key, value, state.cache_ttl)
        {:reply, {:ok, secret}, state}

      {:error, _} = err ->
        {:reply, err, state}
    end
  end

  @impl true
  def handle_call({:delete, key, env}, _from, state) do
    env = env || state.default_env
    path = secret_path(state, env, key)

    case request(state, "DELETE", path) do
      {:ok, _} -> {:reply, :ok, state}
      {:error, _} = err -> {:reply, err, state}
    end
  end

  @impl true
  def handle_call({:list_keys, env}, _from, state) do
    env = env || state.default_env
    path = secrets_path(state, env)

    case request(state, "GET", path) do
      {:ok, %{"keys" => keys}} -> {:reply, {:ok, keys}, state}
      {:error, _} = err -> {:reply, err, state}
    end
  end

  @impl true
  def handle_call(:healthy, _from, state) do
    start = System.monotonic_time(:millisecond)

    result =
      case request(state, "GET", "/me") do
        {:ok, _} ->
          {:ok,
           %{
             ok: true,
             latency_ms: System.monotonic_time(:millisecond) - start,
             last_refresh: state.last_refresh
           }}

        {:error, _} ->
          {:ok,
           %{
             ok: false,
             latency_ms: System.monotonic_time(:millisecond) - start,
             last_refresh: state.last_refresh
           }}
      end

    {:reply, result, state}
  end

  # --- Private helpers ---

  defp secrets_path(state, env) do
    "/orgs/#{state.org_id}/projects/#{state.project_id}/envs/#{env}/secrets"
  end

  defp secret_path(state, env, key) do
    "#{secrets_path(state, env)}/#{URI.encode(key)}"
  end

  defp fetch_all_secrets(state, env) do
    case request(state, "GET", secrets_path(state, env)) do
      {:ok, %{"keys" => keys}} ->
        secrets =
          keys
          |> Enum.map(fn %{"key" => k} -> k end)
          |> Enum.chunk_every(20)
          |> Enum.flat_map(fn batch ->
            batch
            |> Enum.map(fn key ->
              Task.async(fn -> {key, fetch_secret(state, env, key)} end)
            end)
            |> Enum.map(&Task.await(&1, state.timeout + 1_000))
          end)
          |> Enum.filter(fn {_k, result} -> match?({:ok, _}, result) end)
          |> Enum.into(%{}, fn {k, {:ok, v}} -> {k, v} end)

        {:ok, secrets}

      {:error, _} = err ->
        err
    end
  end

  defp fetch_secret(state, env, key) do
    case request(state, "GET", secret_path(state, env, key)) do
      {:ok, %{"secret" => %{"value" => value}}} -> {:ok, value}
      {:error, _} = err -> err
    end
  end

  defp request(state, method, path, body \\ nil) do
    url = "#{state.base_url}/v1/cloud#{path}"

    headers = [
      {"authorization", "Bearer #{state.token}"},
      {"content-type", "application/json"},
      {"user-agent", "zvault-elixir/0.1.0"}
    ]

    do_request_with_retry(method, url, headers, body, state.max_retries, 0, state)
  end

  defp do_request_with_retry(_method, _url, _headers, _body, max_retries, attempt, _state)
       when attempt > max_retries do
    {:error, :max_retries_exceeded}
  end

  defp do_request_with_retry(method, url, headers, body, max_retries, attempt, state) do
    req_opts = [recv_timeout: state.timeout]

    result =
      case method do
        "GET" -> HTTPoison.get(url, headers, req_opts)
        "PUT" -> HTTPoison.put(url, body || "", headers, req_opts)
        "POST" -> HTTPoison.post(url, body || "", headers, req_opts)
        "DELETE" -> HTTPoison.delete(url, headers, req_opts)
      end

    case result do
      {:ok, %HTTPoison.Response{status_code: code, body: resp_body}} when code in 200..299 ->
        if code == 204 do
          {:ok, nil}
        else
          case Jason.decode(resp_body) do
            {:ok, decoded} -> {:ok, decoded}
            {:error, _} -> {:ok, resp_body}
          end
        end

      {:ok, %HTTPoison.Response{status_code: code}} when code in [401, 403] ->
        {:error, {:auth_error, code}}

      {:ok, %HTTPoison.Response{status_code: 404}} ->
        {:error, :not_found}

      {:ok, %HTTPoison.Response{status_code: code}} when code in [429, 500, 502, 503, 504] ->
        if attempt < max_retries do
          delay = @retry_base_delay * :math.pow(2, attempt) |> round()
          jitter = :rand.uniform(round(delay * 0.3))
          log(state, "Retry #{attempt + 1}/#{max_retries} after #{delay + jitter}ms (#{code})")
          Process.sleep(delay + jitter)
          do_request_with_retry(method, url, headers, body, max_retries, attempt + 1, state)
        else
          {:error, {:api_error, code}}
        end

      {:error, %HTTPoison.Error{reason: reason}} ->
        if attempt < max_retries do
          delay = @retry_base_delay * :math.pow(2, attempt) |> round()
          Process.sleep(delay)
          do_request_with_retry(method, url, headers, body, max_retries, attempt + 1, state)
        else
          {:error, {:network_error, reason}}
        end
    end
  end

  defp cache_put(table, env, key, value, ttl) do
    expires_at = System.monotonic_time(:millisecond) + ttl
    :ets.insert(table, {{env, key}, value, expires_at})
  end

  defp cache_put_all(table, env, secrets, ttl) do
    expires_at = System.monotonic_time(:millisecond) + ttl

    Enum.each(secrets, fn {k, v} ->
      :ets.insert(table, {{env, k}, v, expires_at})
    end)
  end

  defp cache_get(table, env, key) do
    case :ets.lookup(table, {env, key}) do
      [{{^env, ^key}, value, expires_at}] ->
        if System.monotonic_time(:millisecond) < expires_at do
          {:ok, value}
        else
          :ets.delete(table, {env, key})
          :miss
        end

      [] ->
        :miss
    end
  end

  defp cache_get_all(table, env) do
    now = System.monotonic_time(:millisecond)

    :ets.foldl(
      fn
        {{^env, key}, value, expires_at}, acc when expires_at > now ->
          Map.put(acc, key, value)

        _, acc ->
          acc
      end,
      %{},
      table
    )
  end

  defp log(%{debug: true}, message) do
    IO.puts("[zvault-sdk] #{message}")
  end

  defp log(_, _), do: :ok
end
