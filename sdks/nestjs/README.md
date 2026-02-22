# @zvault/nestjs

ZVault module for NestJS — decorator-based secret injection.

## Install

```bash
npm install @zvault/nestjs
```

## Setup

```ts
// app.module.ts
import { ZVaultModule } from '@zvault/nestjs';

@Module({
  imports: [
    ZVaultModule.forRoot({ env: 'production' }),
  ],
})
export class AppModule {}
```

## Usage

### Decorator Injection

```ts
import { Injectable } from '@nestjs/common';
import { InjectSecret } from '@zvault/nestjs';

@Injectable()
export class DatabaseService {
  constructor(
    @InjectSecret('DATABASE_URL') private dbUrl: string,
  ) {}
}
```

### Service Injection

```ts
import { Injectable } from '@nestjs/common';
import { ZVaultService } from '@zvault/nestjs';

@Injectable()
export class PaymentService {
  constructor(private vault: ZVaultService) {}

  async getStripeKey() {
    return this.vault.get('STRIPE_KEY');
  }

  async getAllSecrets() {
    return this.vault.getAll();
  }
}
```

## Configuration

| Option | Env Var | Default |
|--------|---------|---------|
| `token` | `ZVAULT_TOKEN` | — (required) |
| `orgId` | `ZVAULT_ORG_ID` | — (required) |
| `projectId` | `ZVAULT_PROJECT_ID` | — (required) |
| `env` | `ZVAULT_ENV` | `production` |
| `url` | `ZVAULT_URL` | `https://api.zvault.cloud` |
| `cacheTtl` | — | `300000` (5 min) |
| `eagerLoad` | — | `true` |

## License

MIT
