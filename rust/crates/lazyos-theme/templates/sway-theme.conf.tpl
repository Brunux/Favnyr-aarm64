{{ header }}
# The theme's two sway surfaces: the wallpaper and the window frame.
#
# It is a user drop-in because ~/.config/sway/config includes
# ~/.config/sway/config.d/*.conf after /etc/sway/config - the last `output bg`
# wins, and the client colours here replace the ones /etc used to pin.
output * bg {{ home }}/Pictures/Wallpapers/{{ wallpaper }} fill

# client.focused <border> <background> <text> <indicator> <child-border>
client.focused          {{ accent }} {{ surface }} {{ strong }} {{ warn }} {{ accent }}
client.focused_inactive {{ dim }} {{ surface }} {{ text }} {{ dim }} {{ dim }}
client.unfocused        {{ surface }} {{ background }} {{ faint }} {{ surface }} {{ surface }}
client.urgent           {{ danger }} {{ danger }} {{ strong }} {{ danger }} {{ danger }}
