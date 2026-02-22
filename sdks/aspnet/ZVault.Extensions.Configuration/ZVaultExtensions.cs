using Microsoft.Extensions.Configuration;

namespace ZVault.Extensions.Configuration;

/// <summary>
/// Extension methods for adding ZVault as a configuration source.
/// </summary>
public static class ZVaultExtensions
{
    /// <summary>
    /// Adds ZVault Cloud as a configuration source.
    /// </summary>
    /// <example>
    /// <code>
    /// var builder = WebApplication.CreateBuilder(args);
    /// builder.Configuration.AddZVault(options =>
    /// {
    ///     options.Org = "my-company";
    ///     options.Project = "my-saas";
    ///     options.Environment = "production";
    /// });
    /// </code>
    /// </example>
    public static IConfigurationBuilder AddZVault(
        this IConfigurationBuilder builder,
        Action<ZVaultConfigurationSource>? configure = null)
    {
        var source = new ZVaultConfigurationSource();
        configure?.Invoke(source);
        builder.Add(source);
        return builder;
    }

    /// <summary>
    /// Adds ZVault Cloud as an optional configuration source (won't throw on failure).
    /// </summary>
    public static IConfigurationBuilder AddZVaultOptional(
        this IConfigurationBuilder builder,
        Action<ZVaultConfigurationSource>? configure = null)
    {
        var source = new ZVaultConfigurationSource { Optional = true };
        configure?.Invoke(source);
        builder.Add(source);
        return builder;
    }
}
