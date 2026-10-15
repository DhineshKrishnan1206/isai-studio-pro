import QtQuick
import Isai

// The page background with soft colour glows: what glass panels blur, so the
// glass has colour to show. Only plain background when effects are off.
Item {
    id: ambient
    Rectangle { anchors.fill: parent; color: Theme.bg }
    Glow {
        visible: Theme.glows
        size: Math.max(ambient.width, ambient.height) * 0.75
        x: -size * 0.3
        y: -size * 0.35
        color: Theme.glowA
        strength: Theme.dark ? 0.32 : 0.2
    }
    Glow {
        visible: Theme.glows
        size: Math.max(ambient.width, ambient.height) * 0.7
        x: ambient.width - size * 0.65
        y: ambient.height - size * 0.6
        color: Theme.glowB
        strength: Theme.dark ? 0.26 : 0.16
    }
    Glow {
        visible: Theme.glows
        size: Math.max(ambient.width, ambient.height) * 0.45
        x: ambient.width * 0.55 - size / 2
        y: -size * 0.25
        color: Theme.accent2
        strength: Theme.dark ? 0.14 : 0.1
    }
}
