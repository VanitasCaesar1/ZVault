package com.zvault.spring;

import org.springframework.core.env.EnumerablePropertySource;

import java.util.Map;

/**
 * Spring PropertySource backed by ZVault secrets.
 * Secrets are available as {@code zvault.SECRET_KEY} properties.
 */
public class ZVaultPropertySource extends EnumerablePropertySource<Map<String, String>> {

    private final Map<String, String> secrets;

    public ZVaultPropertySource(String name, Map<String, String> secrets) {
        super(name, secrets);
        this.secrets = secrets;
    }

    @Override
    public String[] getPropertyNames() {
        return secrets.keySet().stream()
                .map(k -> "zvault." + k)
                .toArray(String[]::new);
    }

    @Override
    public Object getProperty(String name) {
        if (name.startsWith("zvault.")) {
            return secrets.get(name.substring(7));
        }
        return null;
    }
}
