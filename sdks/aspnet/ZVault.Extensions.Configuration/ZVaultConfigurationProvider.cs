using System.Net.Http.Headers;
using System.Text.Json;
using Microsoft.Extensions.Configuration;

namespace ZVault.Extensions.Configuration;

/// <summary>
/// Configuration provider that fetches secrets from ZVault Cloud
/// and exposes them as IConfiguration key-value pairs.
/// </summary>
public class ZVaultConfigurationProvider : ConfigurationProvider, IDisposable
{
    private readonly ZVaultConfigurationSource _source;
    private readonly HttpClient _http;
    private readonly CancellationTokenSource _cts = new();
    private Task? _refreshTask;
    private bool _disposed;

    public ZVaultConfigurationProvider(ZVaultConfigurationSource source)
    {
        _source = source;
        _http = new HttpClient
        {
            BaseAddress = new Uri(source.BaseUrl.TrimEnd('/')),
            Timeout = TimeSpan.FromSeconds(10),
        };

        var token = source.Token
            ?? System.Environment.GetEnvironmentVariable("ZVAULT_TOKEN")
            ?? throw new InvalidOperationException(
                "ZVault token not configured. Set Token or ZVAULT_TOKEN env var.");

        _http.DefaultRequestHeaders.Authorization =
            new AuthenticationHeaderValue("Bearer", token);
        _http.DefaultRequestHeaders.Accept.Add(
            new MediaTypeWithQualityHeaderValue("application/json"));
    }

    public override void Load()
    {
        LoadAsync(CancellationToken.None).ConfigureAwait(false).GetAwaiter().GetResult();

        if (_source.RefreshInterval.HasValue && _refreshTask == null)
        {
            _refreshTask = RefreshLoopAsync(_cts.Token);
        }
    }

    private async Task LoadAsync(CancellationToken ct)
    {
        try
        {
            var org = _source.Org
                ?? System.Environment.GetEnvironmentVariable("ZVAULT_ORG")
                ?? throw new InvalidOperationException("ZVault Org not configured.");
            var project = _source.Project
                ?? System.Environment.GetEnvironmentVariable("ZVAULT_PROJECT")
                ?? throw new InvalidOperationException("ZVault Project not configured.");

            var path = $"/v1/cloud/orgs/{org}/projects/{project}/secrets?environment={_source.Environment}";
            var response = await _http.GetAsync(path, ct).ConfigureAwait(false);
            response.EnsureSuccessStatusCode();

            var json = await response.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
            var doc = JsonDocument.Parse(json);

            var data = new Dictionary<string, string?>(StringComparer.OrdinalIgnoreCase);

            if (doc.RootElement.TryGetProperty("secrets", out var secrets))
            {
                foreach (var secret in secrets.EnumerateArray())
                {
                    var key = secret.GetProperty("key").GetString();
                    var value = secret.GetProperty("value").GetString();
                    if (key == null) continue;

                    var configKey = string.IsNullOrEmpty(_source.Prefix)
                        ? key
                        : $"{_source.Prefix}{key}";

                    // Support nested keys: DB__HOST → DB:HOST (IConfiguration convention)
                    configKey = configKey.Replace("__", ConfigurationPath.KeyDelimiter);
                    data[configKey] = value;
                }
            }

            Data = data;
        }
        catch when (_source.Optional)
        {
            // Silently fall back to empty configuration.
        }
    }

    private async Task RefreshLoopAsync(CancellationToken ct)
    {
        while (!ct.IsCancellationRequested)
        {
            try
            {
                await Task.Delay(_source.RefreshInterval!.Value, ct).ConfigureAwait(false);
                await LoadAsync(ct).ConfigureAwait(false);
                OnReload();
            }
            catch (OperationCanceledException)
            {
                break;
            }
            catch
            {
                // Swallow refresh errors — keep serving last-known values.
            }
        }
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
        _cts.Cancel();
        _cts.Dispose();
        _http.Dispose();
        GC.SuppressFinalize(this);
    }
}
