--- ZVault.nvim — Neovim plugin for ZVault secrets management.
--- Provides Telescope picker, inline virtual text, and secret autocomplete.

local M = {}

--- Default configuration.
M.config = {
  token = vim.env.ZVAULT_TOKEN or "",
  base_url = vim.env.ZVAULT_URL or "https://api.zvault.cloud",
  environment = "development",
  auto_refresh = true,
  refresh_interval = 300, -- seconds
  virtual_text = true,
  virtual_text_prefix = "🔐 ",
}

--- Cached secrets metadata (keys only, never values).
local _cache = {}
local _last_fetch = 0

--- Setup the plugin with user configuration.
---@param opts table|nil
function M.setup(opts)
  M.config = vim.tbl_deep_extend("force", M.config, opts or {})

  -- Register commands
  vim.api.nvim_create_user_command("ZVaultList", function()
    M.telescope_picker()
  end, { desc = "Browse ZVault secrets" })

  vim.api.nvim_create_user_command("ZVaultRefresh", function()
    M.refresh_cache()
    vim.notify("ZVault: cache refreshed", vim.log.levels.INFO)
  end, { desc = "Refresh ZVault secret cache" })

  vim.api.nvim_create_user_command("ZVaultEnv", function(cmd)
    M.config.environment = cmd.args
    M.refresh_cache()
    vim.notify("ZVault: switched to " .. cmd.args, vim.log.levels.INFO)
  end, { nargs = 1, desc = "Switch ZVault environment" })

  -- Auto-refresh on timer
  if M.config.auto_refresh and M.config.token ~= "" then
    local timer = vim.uv.new_timer()
    if timer then
      timer:start(0, M.config.refresh_interval * 1000, vim.schedule_wrap(function()
        M.refresh_cache()
      end))
    end
  end
end

--- Fetch secret keys from ZVault Cloud (metadata only, never values).
function M.refresh_cache()
  if M.config.token == "" then
    return
  end

  local cmd = string.format(
    "curl -sf -H 'Authorization: Bearer %s' '%s/v1/cloud/secrets?environment=%s'",
    M.config.token,
    M.config.base_url,
    M.config.environment
  )

  vim.fn.jobstart(cmd, {
    stdout_buffered = true,
    on_stdout = function(_, data)
      local json = table.concat(data, "")
      local ok, parsed = pcall(vim.json.decode, json)
      if ok and parsed and parsed.secrets then
        _cache = {}
        for _, secret in ipairs(parsed.secrets) do
          table.insert(_cache, {
            key = secret.key,
            updated_at = secret.updated_at or "",
          })
        end
        _last_fetch = os.time()
      end
    end,
  })
end

--- Open Telescope picker to browse secrets.
function M.telescope_picker()
  local ok, telescope = pcall(require, "telescope.pickers")
  if not ok then
    vim.notify("ZVault: telescope.nvim required", vim.log.levels.ERROR)
    return
  end

  local finders = require("telescope.finders")
  local conf = require("telescope.config").values
  local actions = require("telescope.actions")
  local action_state = require("telescope.actions.state")

  if #_cache == 0 then
    M.refresh_cache()
    vim.defer_fn(function() M.telescope_picker() end, 500)
    return
  end

  telescope.new({}, {
    prompt_title = "ZVault Secrets (" .. M.config.environment .. ")",
    finder = finders.new_table({
      results = _cache,
      entry_maker = function(entry)
        return {
          value = entry,
          display = entry.key,
          ordinal = entry.key,
        }
      end,
    }),
    sorter = conf.generic_sorter({}),
    attach_mappings = function(prompt_bufnr)
      actions.select_default:replace(function()
        actions.close(prompt_bufnr)
        local selection = action_state.get_selected_entry()
        if selection then
          -- Insert the key at cursor position
          vim.api.nvim_put({ selection.value.key }, "c", true, true)
        end
      end)
      return true
    end,
  }):find()
end

--- Get cached secret keys for completion.
---@return string[]
function M.get_keys()
  local keys = {}
  for _, entry in ipairs(_cache) do
    table.insert(keys, entry.key)
  end
  return keys
end

return M
