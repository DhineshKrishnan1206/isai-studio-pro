import QtQuick
import QtQuick.Effects
import Isai

// A frosted-glass surface for floating panels: it blurs `backdrop` (the item
// behind it), tints it, and adds a soft sheen and two colour glows.
//
// With effects off (Lightweight profile), or without a backdrop, it's a plain
// solid panel, so it costs nothing on low-spec machines.
// Put the panel's content inside it as children.
Item {
    id: glass
    /// What shows through. Must be a sibling-level item that `glass` overlaps.
    property Item backdrop: null
    property real radius: Theme.radiusXl
    /// 0..1: how much of the backdrop shows through the tint.
    property real clarity: Theme.vivid ? 0.5 : 0.42
    property bool glows: true
    default property alias content: inner.data

    readonly property bool live: Theme.glass && backdrop !== null && visible

    // Where the panel sits over the backdrop, refreshed as it moves or resizes.
    // Bumped when shown, so popups that appear somewhere new re-measure.
    property int remeasure: 0
    onVisibleChanged: if (visible) Qt.callLater(() => remeasure++)
    readonly property rect area: {
        x; y; width; height; remeasure; parent ? parent.x + parent.y : 0
        if (!backdrop) return Qt.rect(0, 0, 0, 0)
        const p = glass.mapToItem(backdrop, 0, 0)
        return Qt.rect(p.x, p.y, width, height)
    }

    // 1. The blurred backdrop, clipped to the rounded shape.
    // Kept visible (the blur covers it): MultiEffect can't sample a hidden source.
    ShaderEffectSource {
        id: snapshot
        visible: false
        live: glass.live
        sourceItem: glass.live ? glass.backdrop : null
        sourceRect: glass.area
        width: glass.width
        height: glass.height
    }
    Item {
        id: shape
        anchors.fill: parent
        visible: false
        layer.enabled: true
        layer.smooth: true
        Rectangle { anchors.fill: parent; radius: glass.radius; color: "black"; antialiasing: true }
    }
    MultiEffect {
        anchors.fill: parent
        visible: glass.live
        source: snapshot
        autoPaddingEnabled: false
        blurEnabled: true
        blur: 1.0
        blurMax: 64
        saturation: 0.35
        maskEnabled: true
        maskSource: shape
        maskThresholdMin: 0.5
        maskSpreadAtMin: 1.0
    }

    // 2. Tint: solid when glass is off.
    Rectangle {
        anchors.fill: parent
        radius: glass.radius
        color: glass.live ? Theme.tint(Theme.panel, 1 - glass.clarity) : Theme.panel
    }

    // 3. Colour glows inside the glass.
    Item {
        anchors.fill: parent
        visible: glass.live && glass.glows
        clip: true
        Glow {
            size: Math.max(glass.width, glass.height) * 0.9
            x: -size * 0.35
            y: glass.height - size * 0.55
            color: Theme.glowA
            strength: Theme.dark ? 0.22 : 0.14
        }
        Glow {
            size: Math.max(glass.width, glass.height) * 0.7
            x: glass.width * 0.45
            y: -size * 0.45
            color: Theme.glowB
            strength: Theme.dark ? 0.16 : 0.1
        }
    }

    // 4. Sheen and edge: light catching the top of the glass.
    Rectangle {
        anchors.fill: parent
        radius: glass.radius
        visible: glass.live
        gradient: Gradient {
            GradientStop { position: 0.0; color: Qt.rgba(1, 1, 1, Theme.dark ? 0.07 : 0.35) }
            GradientStop { position: 0.35; color: Qt.rgba(1, 1, 1, 0) }
        }
    }
    Rectangle {
        anchors.fill: parent
        radius: glass.radius
        color: "transparent"
        border.width: 1
        border.color: glass.live ? Qt.rgba(1, 1, 1, Theme.dark ? 0.12 : 0.6) : Theme.lineStrong
    }

    Item {
        id: inner
        anchors.fill: parent
    }
}
