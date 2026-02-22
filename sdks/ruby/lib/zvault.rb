# frozen_string_literal: true

require "net/http"
require "json"
require "uri"

# ZVault Ruby SDK — fetch and cache secrets from ZVault Cloud.
#
# @example
#   vault = ZVault::Client.new(token: ENV["ZVAULT_TOKEN"])
#   secrets = vault.get_all(env: "production")
#   db_url = vault.get("DATABASE_URL", env: "production")
#
module ZVault
  class Error < StandardError; end

  class Client
    DEFAULT_BASE_URL = "https://api.zvault.cloud"
    DEFAULT_CACHE_TTL = 300 # 5 minutes
    DEFAULT_TIMEOUT = 10
    MAX_RETRIES = 2

    # @param token [String] Service token (or ZVAULT_TOKEN env var)
    # @param org_id [String] Organization ID (or ZVAULT_ORG_ID env var)
    # @param project_id [String] Project ID (or ZVAULT_PROJECT_ID env var)
    # @param base_url [String] API base URL
    # @param cache_ttl [Integer] Cache TTL in seconds
    def initialize(token: nil, org_id: nil, project_id: nil, base_url: nil, cache_ttl: nil)
      @token = token || ENV["ZVAULT_TOKEN"] || raise(Error, "token is required")
      @org_id = org_id || ENV["ZVAULT_ORG_ID"] || raise(Error, "org_id is required")
      @project_id = project_id || ENV["ZVAULT_PROJECT_ID"] || raise(Error, "project_id is required")
      @base_url = (base_url || ENV["ZVAULT_URL"] || DEFAULT_BASE_URL).chomp("/")
      @cache_ttl = cache_ttl || DEFAULT_CACHE_TTL
      @cache = {}
      @mutex = Mutex.new
    end

    # Fetch all secrets for the given environment.
    # @param env [String] Environment slug
    # @return [Hash<String, String>]
    def get_all(env: "production")
      @mutex.synchronize do
        entry = @cache[env]
        return entry[:data] if entry && entry[:expires_at] > Time.now

        begin
          url = "#{@base_url}/v1/cloud/orgs/#{@org_id}/projects/#{@project_id}/envs/#{env}/secrets"
          body = fetch_with_retry(url)
          secrets = parse_secrets(body)
          @cache[env] = { data: secrets, expires_at: Time.now + @cache_ttl }
          secrets
        rescue StandardError => e
          return entry[:data] if entry

          raise Error, "Failed to fetch secrets: #{e.message}"
        end
      end
    end

    # Fetch a single secret by key.
    # @param key [String] Secret key
    # @param env [String] Environment slug
    # @return [String]
    def get(key, env: "production")
      all = get_all(env: env)
      all.fetch(key) { raise Error, "Secret not found: #{key}" }
    end

    # Check if the ZVault API is reachable.
    # @return [Boolean]
    def healthy?
      uri = URI("#{@base_url}/health")
      res = Net::HTTP.get_response(uri)
      res.is_a?(Net::HTTPSuccess)
    rescue StandardError
      false
    end

    # Inject all secrets into ENV.
    # @param env [String] Environment slug
    # @return [Integer] Number of secrets injected
    def inject_into_env(env: "production")
      secrets = get_all(env: env)
      count = 0
      secrets.each do |k, v|
        unless ENV.key?(k)
          ENV[k] = v
          count += 1
        end
      end
      count
    end

    private

    def fetch_with_retry(url)
      last_err = nil

      (MAX_RETRIES + 1).times do |i|
        begin
          uri = URI(url)
          http = Net::HTTP.new(uri.host, uri.port)
          http.use_ssl = uri.scheme == "https"
          http.open_timeout = DEFAULT_TIMEOUT
          http.read_timeout = DEFAULT_TIMEOUT

          req = Net::HTTP::Get.new(uri)
          req["Authorization"] = "Bearer #{@token}"
          req["Content-Type"] = "application/json"
          req["User-Agent"] = "zvault-ruby/0.1.0"

          res = http.request(req)

          return res.body if res.is_a?(Net::HTTPSuccess)

          last_err = Error.new("HTTP #{res.code}")
          raise last_err if res.code.to_i < 500 && res.code.to_i != 429
        rescue Error
          raise
        rescue StandardError => e
          last_err = e
        end

        sleep(0.3 * (2**i)) if i < MAX_RETRIES
      end

      raise Error, "Request failed after retries: #{last_err&.message}"
    end

    def parse_secrets(json_str)
      data = JSON.parse(json_str)
      result = {}
      Array(data["secrets"]).each do |s|
        result[s["key"]] = s["value"] if s["key"] && s["value"]
      end
      result.freeze
    end
  end
end
