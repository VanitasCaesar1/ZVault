using System.Collections.Concurrent;
using System.Net.Http.Headers;
using System.Text.Json;

namespace ZVault;

/// <summary>
/// ZVault .NET SDK — fetch and cache secrets from ZVault Cloud.
/// </summary>
/// <example>
/// <code>
/// var vault = new ZVaultClient(Environment.GetEnvironmentVariable("ZVAULT_TOKEN")!)
/// {
///     OrgId = Environment.GetEnvironmentVariable("ZVAULT_ORG_ID")!,
///     ProjectId = Environment.GetEnvironmentVariable("ZVAULT_PROJECT_ID")!,
/// };
///
/// var secrets = await vault.GetAllAsync("production");
/// var dbUrl = await vault.GetAsync("DATABASE_URL", "production");
/// </code>
/// </example>
public sealed class ZVaultClient : IDisposable
{
    private const string DefaultBaseUrl = "https://api.zvault.cloud";
    private static readonly TimeSpan DefaultTimeout = TimeSpan.FromSeconds(10);
    private static readonly TimeSpan DefaultCacheTtl = TimeSpan.FromMinutes(5);
    private const int MaxRetries = 2;

    private readonly HttpClient _http;
    private readonly string _token;
    private readonly ConcurrentDictionary<string, CacheEntry> _cache = new();

    public string OrgId { get; set; } = "";
    public string ProjectId { get; set; } = "";
    public string BaseUrl { get; set; } = DefaultBaseUrl;
    public TimeSpan CacheTtl { get; set; } = DefaultCacheTtl;

    public ZVaultClient(string token)
    {
        _token = token ?? throw new ArgumentNullException(nameof(token));
        _http = new HttpClient { Timeout = DefaultTimeout };
        _http.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", token);
        _http.DefaultRequestHeaders.UserAgent.ParseAdd("zvault-dotnet/0.1.0");
    }

    /// <summary>Fetch all secrets for the given environment.</summary>
    public async Task<IReadOnlyDictionary<string, string>> GetAllAsync(string env, CancellationToken ct = default)
    {
        if (_cache.TryGetValue(env, out var entry) && entry.ExpiresAt > DateTimeOffset.UtcNow)
            return entry.Data;

        try
        {
            var url = $"{BaseUrl.TrimEnd('/')}/v1/cloud/orgs/{OrgId}/projects/{ProjectId}/envs/{env}/secrets";
            var json = await FetchWithRetryAsync(url, ct);
            var secrets = ParseSecrets(json);
            _cache[env] = new CacheEntry(secrets, DateTimeOffset.UtcNow.Add(CacheTtl));
            return secrets;
        }
        catch
        {
            if (entry is not null) return entry.Data;
            throw;
        }
    }

    /// <summary>Fetch a single secret by key.</summary>
    public async Task<string> GetAsync(string key, string env, CancellationToken ct = default)
    {
        var all = await GetAllAsync(env, ct);
        return all.TryGetValue(key, out var value)
            ? value
            : throw new KeyNotFoundException($"Secret not found: {key}");
    }

    /// <summary>Check if the ZVault API is reachable.</summary>
    public async Task<bool> HealthyAsync(CancellationToken ct = default)
    {
        try
        {
            var res = await _http.GetAsync($"{BaseUrl.TrimEnd('/')}/health", ct);
            return res.IsSuccessStatusCode;
        }
        catch
        {
            return false;
        }
    }

    /// <summary>Inject all secrets into environment variables.</summary>
    public async Task<int> InjectIntoEnvAsync(string env, CancellationToken ct = default)
    {
        var secrets = await GetAllAsync(env, ct);
        var count = 0;
        foreach (var (k, v) in secrets)
        {
            if (Environment.GetEnvironmentVariable(k) is null)
            {
                Environment.SetEnvironmentVariable(k, v);
                count++;
            }
        }
        return count;
    }

    private async Task<string> FetchWithRetryAsync(string url, CancellationToken ct)
    {
        Exception? lastErr = null;

        for (var i = 0; i <= MaxRetries; i++)
        {
            try
            {
                var res = await _http.GetAsync(url, ct);
                if (res.IsSuccessStatusCode)
                    return await res.Content.ReadAsStringAsync(ct);

                lastErr = new HttpRequestException($"HTTP {(int)res.StatusCode}");
                if ((int)res.StatusCode < 500 && (int)res.StatusCode != 429)
                    throw new ZVaultException($"HTTP {(int)res.StatusCode}", lastErr);
            }
            catch (ZVaultException) { throw; }
            catch (Exception e) { lastErr = e; }

            if (i < MaxRetries)
                await Task.Delay(300 * (1 << i), ct);
        }

        throw new ZVaultException("Request failed after retries", lastErr);
    }

    private static IReadOnlyDictionary<string, string> ParseSecrets(string json)
    {
        var dict = new Dictionary<string, string>();
        using var doc = JsonDocument.Parse(json);

        if (doc.RootElement.TryGetProperty("secrets", out var arr))
        {
            foreach (var item in arr.EnumerateArray())
            {
                var key = item.GetProperty("key").GetString();
                var value = item.GetProperty("value").GetString();
                if (key is not null && value is not null)
                    dict[key] = value;
            }
        }

        return dict.AsReadOnly();
    }

    public void Dispose() => _http.Dispose();

    private sealed record CacheEntry(IReadOnlyDictionary<string, string> Data, DateTimeOffset ExpiresAt);
}

/// <summary>Exception thrown by ZVault SDK operations.</summary>
public class ZVaultException : Exception
{
    public ZVaultException(string message) : base(message) { }
    public ZVaultException(string message, Exception? inner) : base(message, inner) { }
}
