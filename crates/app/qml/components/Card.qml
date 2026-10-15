import QtQuick
import QtQuick.Effects
import Isai

// A raised surface. `elevated` adds a soft shadow when effects are on.
Rectangle {
    property bool elevated: false

    radius: Theme.radiusLg
    color: Theme.surface
    border.color: Theme.line
    border.width: 1
    Behavior on color { ColorAnimation { duration: Theme.fast } }

    layer.enabled: elevated && Theme.effects
    layer.effect: MultiEffect {
        shadowEnabled: true
        shadowColor: Theme.shadow
        shadowBlur: 1.0
        blurMax: 48
        shadowVerticalOffset: 12
    }
}
