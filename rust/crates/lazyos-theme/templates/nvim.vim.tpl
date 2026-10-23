{{ header:" }}
" The session colourscheme: `:colorscheme lazyos`. Only the groups the editor
" shows while reading and writing are set - a full groups list is what a plugin
" like kanagawa is for, and this file is generated, so a hand edit is lost on
" the next theme switch.
set background=dark
highlight clear
if exists("syntax_on")
  syntax reset
endif
let g:colors_name = "lazyos"

hi Normal       guifg={{ text }} guibg={{ background }} ctermfg=250 ctermbg=234
hi NonText      guifg={{ dim }}
hi Comment      guifg={{ dim }} gui=italic
hi Constant     guifg={{ accent }}
hi String       guifg={{ ok }}
hi Identifier   guifg={{ accent_alt }} gui=none
hi Function     guifg={{ accent_alt }} gui=bold
hi Statement    guifg={{ warn }} gui=none
hi PreProc      guifg={{ accent }}
hi Type         guifg={{ ok }} gui=none
hi Special      guifg={{ warn }}
hi Underlined   guifg={{ accent }} gui=underline
hi Error        guifg={{ danger }} gui=bold
hi Todo         guifg={{ warn }} guibg={{ surface }} gui=bold
hi Visual       guibg={{ selection }}
hi CursorLine   guibg={{ surface }}
hi CursorLineNr guifg={{ accent }} gui=bold
hi LineNr       guifg={{ dim }}
hi SignColumn   guibg={{ background }}
hi Pmenu        guifg={{ text }} guibg={{ surface }}
hi PmenuSel     guifg={{ strong }} guibg={{ selection }} gui=bold
hi StatusLine   guifg={{ strong }} guibg={{ surface }} gui=none
hi StatusLineNC guifg={{ dim }} guibg={{ surface }}
hi VertSplit    guifg={{ surface_alt }} guibg={{ background }}
hi TabLine      guifg={{ dim }} guibg={{ surface }}
hi TabLineSel   guifg={{ strong }} guibg={{ selection }}
hi Search       guifg={{ background }} guibg={{ warn }}
hi IncSearch    guifg={{ background }} guibg={{ accent }}
hi MatchParen   guifg={{ warn }} gui=bold
hi DiffAdd      guibg={{ surface }} guifg={{ ok }}
hi DiffChange   guibg={{ surface }} guifg={{ warn }}
hi DiffDelete   guibg={{ surface }} guifg={{ danger }}
hi DiagnosticError guifg={{ danger }}
hi DiagnosticWarn  guifg={{ warn }}
hi DiagnosticInfo  guifg={{ info }}
hi DiagnosticHint  guifg={{ ok }}
