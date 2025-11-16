-- Simple Neovim setup module for the Shrimpl language server.
-- Usage (in init.lua):
--   require("shrimpl").setup()

local M = {}

function M.setup()
  local ok, lspconfig = pcall(require, "lspconfig")
  if not ok then
    vim.notify("[shrimpl] nvim-lspconfig not found", vim.log.levels.ERROR)
    return
  end

  -- Define a custom server config called "shrimpl_ls"
  lspconfig.shrimpl_ls = {
    default_config = {
      cmd = { "shrimpl-lsp" }, -- Override here if needed
      filetypes = { "shrimpl" },
      root_dir = function(fname)
        local util = require("lspconfig.util")
        return util.root_pattern("app.shr", "Cargo.toml", ".git")(fname)
          or util.path.dirname(fname)
      end,
      settings = {},
    },
  }

  -- Actually start the server for opened buffers
  lspconfig.shrimpl_ls.setup({})
end

return M
