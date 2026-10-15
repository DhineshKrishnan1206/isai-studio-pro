import QtQuick
import Isai

// Progress spinner. It keeps turning even with animations off, because it
// tells the user that work is happening.
Icon {
    glyph: Icons.circleNotch
    size: 18
    color: Theme.accent
    RotationAnimator on rotation {
        from: 0; to: 360
        duration: 900
        loops: Animation.Infinite
        running: visible
    }
}
