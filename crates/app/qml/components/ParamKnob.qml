import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Shapes
import Isai

// One effect parameter: a knob with its name and value underneath, or a
// small step switch for parameters with named choices (e.g. delay sync).
//
// `def` is a parameter from `effectKindsJson` ({ key, label, min, max,
// default, unit, log, choices }). Drag up/down (⇧ for fine), scroll, or
// double-click for the default. `moved(value, done)`: done ends the change.
Item {
    id: knob
    property var def
    property real value: def ? def.default : 0
    property color tint: Theme.accent
    /// Shown under the knob (the parameter's own label when empty).
    property string label: ""
    signal moved(real value, bool done)

    property real liveValue: 0
    readonly property real shown: area.pressed ? liveValue : value
    readonly property bool stepped: !!def && def.choices.length > 0
    /// Knobs for ± values (gain) light up from the centre.
    readonly property bool bipolar: !!def && def.min < 0 && def.max > 0

    implicitWidth: 54
    implicitHeight: 64

    function posOf(v) {
        if (!def) return 0
        const x = Math.max(def.min, Math.min(def.max, v))
        if (def.log && def.min > 0) return Math.log(x / def.min) / Math.log(def.max / def.min)
        return (x - def.min) / (def.max - def.min)
    }
    function valueOf(p) {
        p = Math.max(0, Math.min(1, p))
        if (def.log && def.min > 0) return def.min * Math.pow(def.max / def.min, p)
        return def.min + p * (def.max - def.min)
    }
    function tidy(v) {
        if (stepped) return Math.round(v)
        const span = def.max - def.min
        const step = def.unit === "Hz" ? (v >= 1000 ? 10 : 1) : span > 200 ? 1 : span > 20 ? 0.1 : 0.01
        return Math.round(v / step) * step
    }
    function emit(v, done) {
        liveValue = tidy(Math.max(def.min, Math.min(def.max, v)))
        moved(liveValue, done)
    }
    function text(v) {
        if (!def) return ""
        if (stepped) return def.choices[Math.round(v)] || ""
        switch (def.unit) {
        case "Hz": return v >= 1000 ? (v / 1000).toFixed(v >= 10000 ? 1 : 2) + "k" : Math.round(v) + " Hz"
        case "dB": return (v > 0.05 ? "+" : "") + v.toFixed(1) + " dB"
        case "ms": return v < 10 ? v.toFixed(1) + " ms" : Math.round(v) + " ms"
        case "%": return Math.round(v) + "%"
        case ":1": return v.toFixed(1) + ":1"
        case "ct": return v.toFixed(1) + " ct"
        case "s": return v < 10 ? v.toFixed(2) + " s" : v.toFixed(1) + " s"
        default: return v.toFixed(2)
        }
    }

    readonly property real pos: posOf(shown)
    readonly property real zeroPos: bipolar ? posOf(0) : 0

    // Knob (or step switch)
    Item {
        id: dial
        width: 30; height: 30
        anchors.horizontalCenter: parent.horizontalCenter
        visible: !knob.stepped
        Shape {
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer
            ShapePath {
                strokeColor: Theme.lineStrong
                strokeWidth: 3
                fillColor: "transparent"
                capStyle: ShapePath.RoundCap
                PathAngleArc { centerX: 15; centerY: 15; radiusX: 12; radiusY: 12; startAngle: 135; sweepAngle: 270 }
            }
            ShapePath {
                strokeColor: knob.tint
                strokeWidth: 3
                fillColor: "transparent"
                capStyle: ShapePath.RoundCap
                PathAngleArc {
                    centerX: 15; centerY: 15; radiusX: 12; radiusY: 12
                    startAngle: 135 + 270 * Math.min(knob.zeroPos, knob.pos)
                    sweepAngle: 270 * Math.abs(knob.pos - knob.zeroPos)
                }
            }
        }
        Rectangle {
            x: 14; y: 6
            width: 2; height: 8; radius: 1
            color: Theme.text
            transform: Rotation { origin.x: 1; origin.y: 9; angle: -135 + 270 * knob.pos }
        }
    }
    AbstractButton {
        visible: knob.stepped
        anchors.horizontalCenter: parent.horizontalCenter
        y: 3
        width: parent.width - 4
        height: 24
        hoverEnabled: true
        // Click for the next choice, right-click for the previous.
        onClicked: knob.emit((Math.round(knob.value) + 1) % knob.def.choices.length, true)
        background: Rectangle {
            radius: Theme.radiusSm
            color: parent.hovered ? Theme.hover : Theme.tint(Theme.sunken, 0.7)
            border.color: Theme.line
        }
        contentItem: DText {
            text: knob.text(knob.value)
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
            font.pixelSize: Theme.textXs
            elide: Text.ElideRight
        }
    }

    MouseArea {
        id: area
        visible: !knob.stepped
        anchors.fill: dial
        anchors.margins: -4
        hoverEnabled: true
        preventStealing: true
        cursorShape: Qt.SizeVerCursor
        property real startY
        property real startPos
        onPressed: m => { knob.liveValue = knob.value; startY = m.y; startPos = knob.posOf(knob.value) }
        onPositionChanged: m => {
            if (!pressed) return
            const scale = (m.modifiers & Qt.ShiftModifier) ? 0.2 : 1
            knob.emit(knob.valueOf(startPos + (startY - m.y) / 120 * scale), false)
        }
        onReleased: knob.emit(knob.liveValue, true)
        onDoubleClicked: knob.emit(knob.def.default, true)
        onWheel: w => {
            const d = (w.angleDelta.y !== 0 ? w.angleDelta.y / 120 : w.pixelDelta.y / 30) * 0.02
            knob.emit(knob.valueOf(knob.posOf(knob.value) + d), true)
        }
    }

    DText {
        id: valueText
        anchors.horizontalCenter: parent.horizontalCenter
        y: 32
        text: knob.stepped ? "" : knob.text(knob.shown)
        font.family: Theme.mono
        font.pixelSize: 10
        color: area.pressed ? Theme.text : Theme.textDim
    }
    DText {
        anchors.horizontalCenter: parent.horizontalCenter
        y: 46
        width: parent.width
        horizontalAlignment: Text.AlignHCenter
        text: knob.label || (knob.def ? knob.def.label : "")
        elide: Text.ElideRight
        font.pixelSize: 10
        faint: true
    }
    DToolTip {
        visible: area.containsMouse && !area.pressed
        text: (knob.def ? knob.def.label : "") + " · double-click to reset, ⇧ to fine-tune"
    }
}
