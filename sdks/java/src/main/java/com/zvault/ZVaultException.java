package com.zvault;

/**
 * Exception thrown by ZVault SDK operations.
 */
public class ZVaultException extends Exception {

    public ZVaultException(String message) {
        super(message);
    }

    public ZVaultException(String message, Throwable cause) {
        super(message, cause);
    }
}
