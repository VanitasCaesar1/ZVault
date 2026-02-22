package com.zvault.spring;

import org.springframework.boot.context.properties.ConfigurationProperties;

/**
 * Configuration properties for ZVault Spring Boot Starter.
 *
 * <pre>
 * zvault:
 *   token: ${ZVAULT_TOKEN}
 *   org-id: ${ZVAULT_ORG_ID}
 *   project-id: ${ZVAULT_PROJECT_ID}
 *   env: production
 *   base-url: https://api.zvault.cloud
 *   cache-ttl: 300
 *   enabled: true
 * </pre>
 */
@ConfigurationProperties(prefix = "zvault")
public class ZVaultProperties {

    private String token = "";
    private String orgId = "";
    private String projectId = "";
    private String env = "production";
    private String baseUrl = "https://api.zvault.cloud";
    private int cacheTtl = 300;
    private boolean enabled = true;

    public String getToken() { return token; }
    public void setToken(String token) { this.token = token; }

    public String getOrgId() { return orgId; }
    public void setOrgId(String orgId) { this.orgId = orgId; }

    public String getProjectId() { return projectId; }
    public void setProjectId(String projectId) { this.projectId = projectId; }

    public String getEnv() { return env; }
    public void setEnv(String env) { this.env = env; }

    public String getBaseUrl() { return baseUrl; }
    public void setBaseUrl(String baseUrl) { this.baseUrl = baseUrl; }

    public int getCacheTtl() { return cacheTtl; }
    public void setCacheTtl(int cacheTtl) { this.cacheTtl = cacheTtl; }

    public boolean isEnabled() { return enabled; }
    public void setEnabled(boolean enabled) { this.enabled = enabled; }
}
