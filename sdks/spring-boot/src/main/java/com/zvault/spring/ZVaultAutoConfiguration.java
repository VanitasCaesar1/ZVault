package com.zvault.spring;

import com.zvault.ZVault;
import com.zvault.ZVaultException;
import org.springframework.boot.autoconfigure.condition.ConditionalOnProperty;
import org.springframework.boot.context.properties.EnableConfigurationProperties;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.core.env.ConfigurableEnvironment;

import java.util.Map;
import java.util.logging.Logger;

/**
 * Auto-configuration for ZVault Spring Boot Starter.
 * Registers a ZVault client bean and injects secrets as Spring properties.
 */
@Configuration
@EnableConfigurationProperties(ZVaultProperties.class)
@ConditionalOnProperty(prefix = "zvault", name = "enabled", havingValue = "true", matchIfMissing = true)
public class ZVaultAutoConfiguration {

    private static final Logger LOG = Logger.getLogger(ZVaultAutoConfiguration.class.getName());

    @Bean
    public ZVault zvaultClient(ZVaultProperties props, ConfigurableEnvironment env) {
        ZVault client = ZVault.builder()
                .token(props.getToken())
                .orgId(props.getOrgId())
                .projectId(props.getProjectId())
                .baseUrl(props.getBaseUrl())
                .build();

        try {
            Map<String, String> secrets = client.getAll(props.getEnv());
            env.getPropertySources().addFirst(
                    new ZVaultPropertySource("zvault", secrets)
            );
            LOG.info("[zvault] Loaded " + secrets.size() + " secrets from '" + props.getEnv() + "'");
        } catch (ZVaultException e) {
            LOG.warning("[zvault] Failed to load secrets: " + e.getMessage());
        }

        return client;
    }
}
