using Microsoft.Extensions.Configuration;

namespace ZVault.Extensions.Configuration;

/// <summary>
/// Configuration source that loads secrets from ZVault Cloud.
/// </summary>
public class ZVaultConfigurationSource : IConfigurationSource
{
    /// <summary>ZVault Cloud API base URL.</summary>
    public string BaseUrl { get; set; } = "https://api.zvault.cloud";

    /// <summary>Service token (zvt_xxx) or ZVAULT_TOKEN env var.</summary>
    public string? Token { get; set; }

    /// <summary>Environment to load secrets from (e.g., "production").</summary>
    public string Environment { get; set; } = "development";

    /// <summary>Organization slug.</summary>
    public string? Org { get; set; }

    /// <summary>Project slug.</summary>
    public string? Project { get; set; }

    /// <summary>Prefix to prepend to all keys (e.g., "ZVault:").</summary>
    public string? Prefix { get; set; }

    /// <summary>Auto-refresh interval. Set to null to disable.</summary>
    public TimeSpan? RefreshInterval { get; set; } = TimeSpan.FromMinutes(5);

    /// <summary>Whether to throw on load failure or silently fall back to empty.</summary>
    public bool Optional { get; set; } = false;

    public IConfigurationProvider Build(IConfigurationBuilder builder)
    {
        return new ZVaultConfigurationProvider(this);
    }
}
