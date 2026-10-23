{{ header }}
# lazygit's colours (lazygit >= 0.40 reads `gui.theme`, one key per element).
# The rest of lazygit's behaviour is left to its defaults on purpose: this file
# owns the palette and nothing else, so a theme switch cannot change a keybinding.
gui:
  theme:
    activeBorderColor:
      - "{{ accent }}"
      - bold
    inactiveBorderColor:
      - "{{ surface_alt }}"
    searchingActiveBorderColor:
      - "{{ accent_alt }}"
      - bold
    optionsTextColor:
      - "{{ accent_alt }}"
    selectedLineBgColor:
      - "{{ selection }}"
    cherryPickedCommitBgColor:
      - "{{ accent_alt }}"
    cherryPickedCommitFgColor:
      - "{{ background }}"
    unstagedChangesColor:
      - "{{ danger }}"
    defaultFgColor:
      - "{{ text }}"
