/**
 * @zvault/nestjs — ZVault module for NestJS
 *
 * @example
 * ```ts
 * // app.module.ts
 * import { ZVaultModule } from '@zvault/nestjs';
 *
 * @Module({
 *   imports: [
 *     ZVaultModule.forRoot({ env: 'production' }),
 *   ],
 * })
 * export class AppModule {}
 *
 * // my.service.ts
 * import { InjectSecret, ZVaultService } from '@zvault/nestjs';
 *
 * @Injectable()
 * export class MyService {
 *   constructor(
 *     @InjectSecret('DATABASE_URL') private dbUrl: string,
 *     private vault: ZVaultService,
 *   ) {}
 *
 *   async getStripeKey() {
 *     return this.vault.get('STRIPE_KEY');
 *   }
 * }
 * ```
 */

import {
  Module,
  Global,
  DynamicModule,
  Injectable,
  Inject,
  OnModuleInit,
  OnModuleDestroy,
} from '@nestjs/common';

// ─── Types ───

export interface ZVaultModuleConfig {
  token?: string;
  orgId?: string;
  projectId?: string;
  env?: string;
  url?: string;
  cacheTtl?: number;
  /** Pre-fetch all secrets on module init. Default: true. */
  eagerLoad?: boolean;
}

interface SecretEntry {
  key: string;
  value: string;
}

// ─── Constants ───

const ZVAULT_CONFIG = 'ZVAULT_CONFIG';
const ZVAULT_SECRET_PREFIX = 'ZVAULT_SECRET:';
const DEFAULT_URL = 'https://api.zvault.cloud';
const DEFAULT_CACHE_TTL = 300_000;
const DEFAULT_TIMEOUT = 10_000;

// ─── Decorator ───

/**
 * Inject a ZVault secret by key.
 *
 * @example
 * ```ts
 * constructor(@InjectSecret('DATABASE_URL') private dbUrl: string) {}
 * ```
 */
export function InjectSecret(key: string): ParameterDecorator {
  return Inject(`${ZVAULT_SECRET_PREFIX}${key}`);
}

// ─── Service ───

@Injectable()
export class ZVaultService implements OnModuleInit, OnModuleDestroy {
  private cache = new Map<string, { value: string; expiresAt: number }>();
  private readonly token: string;
  private readonly orgId: string;
  private readonly projectId: string;
  private readonly envSlug: string;
  private readonly baseUrl: string;
  private readonly cacheTtl: number;
  private readonly eagerLoad: boolean;

  constructor(@Inject(ZVAULT_CONFIG) config: ZVaultModuleConfig) {
    this.token = config.token ?? process.env.ZVAULT_TOKEN ?? '';
    this.orgId = config.orgId ?? process.env.ZVAULT_ORG_ID ?? '';
    this.projectId = config.projectId ?? process.env.ZVAULT_PROJECT_ID ?? '';
    this.envSlug = config.env ?? process.env.ZVAULT_ENV ?? 'production';
    this.baseUrl = (config.url ?? process.env.ZVAULT_URL ?? DEFAULT_URL).replace(/\/+$/, '');
    this.cacheTtl = config.cacheTtl ?? DEFAULT_CACHE_TTL;
    this.eagerLoad = config.eagerLoad ?? true;
  }

  async onModuleInit() {
    if (this.eagerLoad && this.token && this.orgId && this.projectId) {
      await this.loadAll();
    }
  }

  onModuleDestroy() {
    this.cache.clear();
  }

  /** Get a single secret value. */
  async get(key: string): Promise<string> {
    const cached = this.cache.get(key);
    if (cached && cached.expiresAt > Date.now()) {
      return cached.value;
    }

    const url = `${this.baseUrl}/v1/cloud/orgs/${this.orgId}/projects/${this.projectId}/envs/${this.envSlug}/secrets/${encodeURIComponent(key)}`;
    const res = await this.request<{ secret: SecretEntry }>(url);
    this.cache.set(key, { value: res.secret.value, expiresAt: Date.now() + this.cacheTtl });
    return res.secret.value;
  }

  /** Get all secrets as a plain object. */
  async getAll(): Promise<Record<string, string>> {
    return this.loadAll();
  }

  private async loadAll(): Promise<Record<string, string>> {
    const keysUrl = `${this.baseUrl}/v1/cloud/orgs/${this.orgId}/projects/${this.projectId}/envs/${this.envSlug}/secrets`;
    const keysRes = await this.request<{ keys: Array<{ key: string }> }>(keysUrl);

    const result: Record<string, string> = {};
    const batch = 20;

    for (let i = 0; i < keysRes.keys.length; i += batch) {
      const chunk = keysRes.keys.slice(i, i + batch);
      const settled = await Promise.allSettled(
        chunk.map((k) =>
          this.request<{ secret: SecretEntry }>(
            `${this.baseUrl}/v1/cloud/orgs/${this.orgId}/projects/${this.projectId}/envs/${this.envSlug}/secrets/${encodeURIComponent(k.key)}`,
          ),
        ),
      );

      for (const entry of settled) {
        if (entry.status === 'fulfilled') {
          const s = entry.value.secret;
          result[s.key] = s.value;
          this.cache.set(s.key, { value: s.value, expiresAt: Date.now() + this.cacheTtl });
        }
      }
    }

    return result;
  }

  private async request<T>(url: string): Promise<T> {
    const controller = new AbortController();
    const tid = setTimeout(() => controller.abort(), DEFAULT_TIMEOUT);

    const res = await fetch(url, {
      headers: {
        Authorization: `Bearer ${this.token}`,
        'User-Agent': '@zvault/nestjs/0.1.0',
      },
      signal: controller.signal,
    });

    clearTimeout(tid);

    if (!res.ok) {
      const body = await res.json().catch(() => null);
      throw new Error(body?.error?.message ?? `ZVault HTTP ${res.status}`);
    }

    return res.json() as Promise<T>;
  }
}

// ─── Module ───

@Global()
@Module({})
export class ZVaultModule {
  /**
   * Register ZVaultModule globally.
   *
   * @example
   * ```ts
   * ZVaultModule.forRoot({ env: 'production' })
   * ```
   */
  static forRoot(config: ZVaultModuleConfig = {}): DynamicModule {
    return {
      module: ZVaultModule,
      providers: [
        { provide: ZVAULT_CONFIG, useValue: config },
        ZVaultService,
      ],
      exports: [ZVaultService],
    };
  }
}
