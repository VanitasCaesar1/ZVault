# zvault.nvim

Neovim plugin for ZVault secrets management.

## Features

- **Telescope Picker**: Browse and insert secret keys with `:ZVaultList`
- **Environment Switching**: `:ZVaultEnv production`
- **Auto-refresh**: Background cache refresh on configurable interval
- **Secret Autocomplete**: Key suggestions (via nvim-cmp source)

## Installation

### lazy.nvim

```lua
{
  "ArcadeLabsInc/zvault.nvim",
  config = function()
    require("zvault").setup({
      token = vim.env.ZVAULT_TOKEN,
      environment = "development",
    })
  end,
}
```

### packer.nvim

```lua
use {
  "ArcadeLabsInc/zvault.nvim",
  config = function()
    require("zvault").setup()
  end,
}
```

## Configuration

```lua
require("zvault").setup({
  token = vim.env.ZVAULT_TOKEN,
  base_url = "https://api.zvault.cloud",
  environment = "development",
  auto_refresh = true,
  refresh_interval = 300, -- seconds
})
```

## Commands

| Command | Description |
|---------|-------------|
| `:ZVaultList` | Open Telescope picker to browse secrets |
| `:ZVaultRefresh` | Manually refresh secret cache |
| `:ZVaultEnv <env>` | Switch environment |

## Keymaps

```lua
vim.keymap.set("n", "<leader>vs", ":ZVaultList<CR>", { desc = "ZVault secrets" })
vim.keymap.set("n", "<leader>ve", ":ZVaultEnv ", { desc = "ZVault switch env" })
```

## Security

- Only secret keys and metadata are cached — never values
- Token is read from `ZVAULT_TOKEN` environment variable
- All API calls use HTTPS
