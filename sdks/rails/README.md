# zvault-rails

ZVault integration for Ruby on Rails — replaces `Rails.application.credentials`.

## Install

```ruby
# Gemfile
gem "zvault-rails"
```

## Quick Start

```ruby
# config/initializers/zvault.rb
ZVault::Rails.load!(env: Rails.env.to_s)

# Anywhere in your app
db_url = ZVault::Rails["DATABASE_URL"]
```

## Environment Variables

```bash
ZVAULT_TOKEN=zvt_your_service_token
ZVAULT_ORG_ID=org_xxx
ZVAULT_PROJECT_ID=proj_xxx
```

## License

MIT
