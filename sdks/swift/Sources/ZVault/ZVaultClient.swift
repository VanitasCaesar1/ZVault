import Foundation

/// ZVault SDK for Swift.
///
/// Fetches secrets from ZVault Cloud at runtime. Caches in-memory,
/// auto-refreshes on TTL, and gracefully degrades if the API is unreachable.
///
/// ```swift
/// let vault = ZVaultClient(token: ProcessInfo.processInfo.environment["ZVAULT_TOKEN"]!)
/// let secrets = try await vault.getAll(env: "production")
/// let dbUrl = secrets["DATABASE_URL"]
/// ```
public final class ZVaultClient: Sendable {
    private let token: String
    private let baseUrl: String
    private let orgId: String
    private let projectId: String
    private let defaultEnv: String
    private let timeout: TimeInterval
    private let maxRetries: Int
    private let debug: Bool
    private let cacheTtl: TimeInterval
    private let cache = SecretCache()
    private let session: URLSession

    public init(
        token: String? = nil,
        baseUrl: String? = nil,
        orgId: String? = nil,
        projectId: String? = nil,
        defaultEnv: String? = nil,
        cacheTtl: TimeInterval = 300,
        timeout: TimeInterval = 10,
        maxRetries: Int = 3,
        debug: Bool = false
    ) {
        self.token = token ?? Self.env("ZVAULT_TOKEN") ?? ""
        self.baseUrl = (baseUrl ?? Self.env("ZVAULT_URL") ?? "https://api.zvault.cloud")
            .trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        self.orgId = orgId ?? Self.env("ZVAULT_ORG_ID") ?? ""
        self.projectId = projectId ?? Self.env("ZVAULT_PROJECT_ID") ?? ""
        self.defaultEnv = defaultEnv ?? Self.env("ZVAULT_ENV") ?? "development"
        self.cacheTtl = cacheTtl
        self.timeout = timeout
        self.maxRetries = maxRetries
        self.debug = debug

        let config = URLSessionConfiguration.default
        config.timeoutIntervalForRequest = timeout
        config.timeoutIntervalForResource = timeout * 2
        self.session = URLSession(configuration: config)

        precondition(!self.token.isEmpty, "Missing token. Set ZVAULT_TOKEN or pass token parameter.")
    }

    /// Fetch all secrets for an environment.
    public func getAll(env: String? = nil) async throws -> [String: String] {
        let envSlug = env ?? defaultEnv
        requireProjectConfig()

        do {
            let keysResponse: KeysResponse = try await request(
                method: "GET",
                path: secretsPath(env: envSlug)
            )

            var secrets: [String: String] = [:]

            // Fetch in batches of 20
            for batch in keysResponse.keys.chunked(into: 20) {
                try await withThrowingTaskGroup(of: (String, String)?.self) { group in
                    for keyEntry in batch {
                        group.addTask {
                            do {
                                let res: SecretResponse = try await self.request(
                                    method: "GET",
                                    path: self.secretPath(env: envSlug, key: keyEntry.key)
                                )
                                return (res.secret.key, res.secret.value)
                            } catch {
                                return nil
                            }
                        }
                    }

                    for try await result in group {
                        if let (key, value) = result {
                            secrets[key] = value
                        }
                    }
                }
            }

            cache.setAll(env: envSlug, secrets: secrets, ttl: cacheTtl)
            log("Fetched \(secrets.count) secrets for env \"\(envSlug)\"")
            return secrets
        } catch {
            let cached = cache.getAll(env: envSlug)
            if !cached.isEmpty {
                log("API unreachable, serving \(cached.count) cached secrets")
                return cached
            }
            throw error
        }
    }

    /// Fetch a single secret by key.
    public func get(key: String, env: String? = nil) async throws -> String {
        let envSlug = env ?? defaultEnv
        requireProjectConfig()

        if let cached = cache.get(env: envSlug, key: key) {
            log("Cache hit for \"\(key)\" in \"\(envSlug)\"")
            return cached
        }

        let res: SecretResponse = try await request(
            method: "GET",
            path: secretPath(env: envSlug, key: key)
        )

        cache.set(env: envSlug, key: key, value: res.secret.value, ttl: cacheTtl)
        return res.secret.value
    }

    /// Set a secret value.
    public func set(key: String, value: String, env: String? = nil, comment: String = "") async throws -> SecretEntry {
        let envSlug = env ?? defaultEnv
        requireProjectConfig()

        let body = ["value": value, "comment": comment]
        let res: SecretResponse = try await request(
            method: "PUT",
            path: secretPath(env: envSlug, key: key),
            body: body
        )

        cache.set(env: envSlug, key: key, value: value, ttl: cacheTtl)
        return res.secret
    }

    /// Delete a secret.
    public func delete(key: String, env: String? = nil) async throws {
        let envSlug = env ?? defaultEnv
        requireProjectConfig()

        let _: EmptyResponse = try await request(
            method: "DELETE",
            path: secretPath(env: envSlug, key: key)
        )
    }

    /// List secret keys (no values).
    public func listKeys(env: String? = nil) async throws -> [KeyEntry] {
        let envSlug = env ?? defaultEnv
        requireProjectConfig()

        let res: KeysResponse = try await request(
            method: "GET",
            path: secretsPath(env: envSlug)
        )
        return res.keys
    }

    /// Health check.
    public func healthy() async -> HealthStatus {
        let start = Date()
        do {
            let _: MeResponse = try await request(method: "GET", path: "/me")
            return HealthStatus(ok: true, latencyMs: Date().timeIntervalSince(start) * 1000)
        } catch {
            return HealthStatus(ok: false, latencyMs: Date().timeIntervalSince(start) * 1000)
        }
    }

    // MARK: - Private

    private func requireProjectConfig() {
        precondition(!orgId.isEmpty, "Missing orgId. Set ZVAULT_ORG_ID or pass orgId parameter.")
        precondition(!projectId.isEmpty, "Missing projectId. Set ZVAULT_PROJECT_ID or pass projectId parameter.")
    }

    private func secretsPath(env: String) -> String {
        "/orgs/\(orgId)/projects/\(projectId)/envs/\(env)/secrets"
    }

    private func secretPath(env: String, key: String) -> String {
        "\(secretsPath(env: env))/\(key.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? key)"
    }

    private func request<T: Decodable>(
        method: String,
        path: String,
        body: [String: String]? = nil,
        attempt: Int = 0
    ) async throws -> T {
        let url = URL(string: "\(baseUrl)/v1/cloud\(path)")!
        var req = URLRequest(url: url)
        req.httpMethod = method
        req.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.setValue("zvault-swift/0.1.0", forHTTPHeaderField: "User-Agent")

        if let body = body {
            req.httpBody = try JSONSerialization.data(withJSONObject: body)
        }

        do {
            let (data, response) = try await session.data(for: req)
            guard let httpResponse = response as? HTTPURLResponse else {
                throw ZVaultError.networkError("Invalid response")
            }

            let statusCode = httpResponse.statusCode

            if (200...299).contains(statusCode) {
                if statusCode == 204 || data.isEmpty {
                    // Return empty for 204
                    return try JSONDecoder().decode(T.self, from: "{}".data(using: .utf8)!)
                }
                return try JSONDecoder().decode(T.self, from: data)
            }

            if statusCode == 401 || statusCode == 403 {
                throw ZVaultError.authError("Authentication failed (\(statusCode))")
            }

            if statusCode == 404 {
                throw ZVaultError.notFound
            }

            if [429, 500, 502, 503, 504].contains(statusCode) && attempt < maxRetries {
                let delay = Double(500 * (1 << attempt)) / 1000.0
                let jitter = Double.random(in: 0...(delay * 0.3))
                log("Retry \(attempt + 1)/\(maxRetries) after \(Int((delay + jitter) * 1000))ms (\(statusCode))")
                try await Task.sleep(nanoseconds: UInt64((delay + jitter) * 1_000_000_000))
                return try await request(method: method, path: path, body: body, attempt: attempt + 1)
            }

            throw ZVaultError.apiError(statusCode, "HTTP \(statusCode)")
        } catch let error as ZVaultError {
            throw error
        } catch {
            if attempt < maxRetries {
                let delay = Double(500 * (1 << attempt)) / 1000.0
                try await Task.sleep(nanoseconds: UInt64(delay * 1_000_000_000))
                return try await request(method: method, path: path, body: body, attempt: attempt + 1)
            }
            throw ZVaultError.networkError(error.localizedDescription)
        }
    }

    private func log(_ message: String) {
        if debug {
            print("[zvault-sdk] \(message)")
        }
    }

    private static func env(_ name: String) -> String? {
        let value = ProcessInfo.processInfo.environment[name]
        return (value?.isEmpty == false) ? value : nil
    }
}

// MARK: - Models

public struct SecretEntry: Codable, Sendable {
    public let key: String
    public let value: String
}

public struct KeyEntry: Codable, Sendable {
    public let key: String
}

public struct HealthStatus: Sendable {
    public let ok: Bool
    public let latencyMs: Double
}

public enum ZVaultError: Error {
    case authError(String)
    case notFound
    case apiError(Int, String)
    case networkError(String)
    case configError(String)
}

// Internal response types
struct SecretResponse: Codable { let secret: SecretEntry }
struct KeysResponse: Codable { let keys: [KeyEntry] }
struct EmptyResponse: Codable {}
struct MeResponse: Codable {}

// MARK: - Cache

final class SecretCache: @unchecked Sendable {
    private var store: [String: (value: String, expiresAt: Date)] = [:]
    private let lock = NSLock()

    func get(env: String, key: String) -> String? {
        lock.lock()
        defer { lock.unlock() }
        let cacheKey = "\(env):\(key)"
        guard let entry = store[cacheKey], entry.expiresAt > Date() else {
            store.removeValue(forKey: "\(env):\(key)")
            return nil
        }
        return entry.value
    }

    func set(env: String, key: String, value: String, ttl: TimeInterval) {
        lock.lock()
        defer { lock.unlock() }
        store["\(env):\(key)"] = (value, Date().addingTimeInterval(ttl))
    }

    func setAll(env: String, secrets: [String: String], ttl: TimeInterval) {
        lock.lock()
        defer { lock.unlock() }
        let expiresAt = Date().addingTimeInterval(ttl)
        for (key, value) in secrets {
            store["\(env):\(key)"] = (value, expiresAt)
        }
    }

    func getAll(env: String) -> [String: String] {
        lock.lock()
        defer { lock.unlock() }
        let now = Date()
        var result: [String: String] = [:]
        for (cacheKey, entry) in store where entry.expiresAt > now {
            if cacheKey.hasPrefix("\(env):") {
                let key = String(cacheKey.dropFirst(env.count + 1))
                result[key] = entry.value
            }
        }
        return result
    }
}

// MARK: - Array Extension

extension Array {
    func chunked(into size: Int) -> [[Element]] {
        stride(from: 0, to: count, by: size).map {
            Array(self[$0..<Swift.min($0 + size, count)])
        }
    }
}
