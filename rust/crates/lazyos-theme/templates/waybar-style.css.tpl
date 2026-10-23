{{ header }}
/* The bar: restrained terminal aesthetic with consistent module spacing. The
   palette comes from the active theme (spec 004) - this file is generated. */
* {
    font-family: "JetBrains Mono", monospace;
    font-size: 12px;
    font-weight: 600;
    min-height: 0;
    border: none;
    border-radius: 0;
    transition: background-color 120ms ease, color 120ms ease;
}

/* The theme's colours, as roles */
/* The bar is a layer-shell surface with alpha < 1 so that swayfx can blur the
   wallpaper behind it (layer_effects "waybar" in
   /etc/sway/config.d/26-swayfx.conf). The cards below keep enough opacity for
   base1 text to stay readable over any wallpaper; the blur shows through the
   bar itself and between the cards. */
window#waybar {
    background: rgba({{ background:rgb }}, 0.71);
    color: {{ text }};
    border-bottom: 1px solid {{ surface }};
}

tooltip {
    background: {{ surface }};
    color: {{ strong }};
    border: 1px solid {{ dim }};
    border-radius: 6px;
    padding: 5px 8px;
}

tooltip label {
    color: {{ strong }};
}

/* Every indicator is the same card, whether it is a built-in module or one of
   the bar-* scripts: equal padding, height and background. */
#custom-menu,
#custom-busy,
#custom-updates,
#tray,
#custom-keyboard-layout,
#custom-bluetooth,
#custom-vnc,
#network,
#pulseaudio,
#backlight,
#idle_inhibitor,
#battery {
    margin: 4px 0;
    padding: 0 9px;
    background: rgba({{ surface:rgb }}, 0.80);
    border: 1px solid rgba({{ hover:rgb }}, 0.9);
    border-radius: 5px;
}

#custom-menu          { color: {{ accent }}; } /* cyan */
#custom-busy          { color: {{ warn }}; } /* yellow: something is happening */
#custom-updates       { color: {{ warn }}; } /* yellow */
#custom-keyboard-layout { color: {{ faint }}; }
#custom-bluetooth     { color: {{ dim }}; }
#custom-bluetooth.on  { color: {{ accent_alt }}; } /* blue */
#custom-vnc           { color: {{ warn }}; } /* yellow: this session is being served */
#network              { color: {{ accent }}; }
#network.disconnected {
    color: {{ danger }};
    border-color: {{ danger }};
}
#pulseaudio           { color: {{ accent_alt }}; } /* violet */
#pulseaudio.muted     { color: {{ danger }}; }
#backlight            { color: {{ warn }}; }
/* The coffee cup is a switch: dim is its off state, yellow is on. */
#idle_inhibitor       { color: {{ dim }}; }
#idle_inhibitor.activated { color: {{ warn }}; } /* yellow: kept awake */
#battery              { color: {{ ok }}; }
#battery.warning      { color: {{ warn }}; }
#battery.critical:not(.charging) {
    color: {{ danger }};
    background: {{ danger_bg }};
    border-color: {{ danger }};
}
#battery.charging,
#battery.plugged { color: {{ accent }}; }

#custom-menu:hover,
#custom-busy:hover,
#custom-updates:hover,
#custom-keyboard-layout:hover,
#custom-bluetooth:hover,
#custom-vnc:hover,
#network:hover,
#pulseaudio:hover,
#backlight:hover,
#idle_inhibitor:hover,
#battery:hover {
    background: rgba({{ hover:rgb }}, 0.86);
}

/* The StatusNotifier icons keep their own colours; only the slot is themed. */
#tray {
    padding: 0 6px;
    color: {{ text }};
}

#tray > .passive {
    -gtk-icon-effect: dim;
}

#tray > .needs-attention {
    background: {{ danger_bg }};
    border-color: {{ danger }};
}

/* A module whose script reports nothing (no upgrades, a single keyboard
   layout) collapses to zero width instead of leaving an empty card on the
   bar.  The class comes from the JSON the script prints. */
#custom-updates.empty,
#custom-busy.empty,
#custom-keyboard-layout.empty,
#custom-vnc.empty {
    padding: 0;
    margin: 0;
    border: none;
    background: transparent;
}

/* The centered clock is plain text, not a rounded module/card. */
#clock {
    color: {{ strong }};
    background: transparent;
    border: none;
    border-radius: 0;
    box-shadow: none;
    padding: 0;
    margin: 0;
}

/* Workspace buttons look like compact terminal tabs. */
#workspaces {
    margin: 4px 2px 4px 0;
    padding: 0;
}

#workspaces button {
    min-width: 22px;
    padding: 0 7px;
    margin: 0 2px;
    background: rgba({{ surface:rgb }}, 0.80);
    color: {{ text }};
    border: 1px solid rgba({{ hover:rgb }}, 0.9);
    border-radius: 5px;
    box-shadow: none;
}

#workspaces button:hover {
    background: {{ dim }};
    color: {{ strong }};
    border-color: {{ faint }};
}

#workspaces button.focused,
#workspaces button.active {
    background: {{ accent }}; /* cyan */
    color: {{ background }};
    border-color: {{ accent }};
    box-shadow: none;
}

#workspaces button.urgent {
    background: {{ danger }}; /* red */
    color: {{ strong }};
    border-color: {{ danger }};
}

#workspaces button label {
    font-size: 12px;
    font-weight: 700;
}

#mode {
    margin: 4px 0;
    padding: 0 9px;
    color: {{ background }};
    background: {{ warn }}; /* yellow */
    border: 1px solid {{ warn }};
    border-radius: 5px;
}
