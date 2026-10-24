{{ header }}
# fzf's colours, sourced by ~/.bashrc. fzf reads FZF_DEFAULT_OPTS from the
# environment; an exported `--color` block is the only way to keep it in one
# place, because fzf has no colour file.
export FZF_DEFAULT_OPTS="
  --color=fg:{{ text }},bg:{{ background }}
  --color=hl:{{ accent }},fg+:{{ strong }},bg+:{{ selection }},hl+:{{ accent }}
  --color=info:{{ warn }},prompt:{{ accent }},pointer:{{ accent_alt }}
  --color=marker:{{ ok }},spinner:{{ accent }},header:{{ dim }}
  --color=border:{{ surface_alt }},label:{{ accent_alt }},query:{{ text }}
  --height=60% --layout=reverse --border=rounded --prompt='  '
"
