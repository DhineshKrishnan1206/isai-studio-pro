import QtQuick
import QtQuick.Shapes
import Isai

// A small knob for a send level, -60 dB (off) to +6 dB; the arc fills from
// the left as the level rises. Drag up/down (⇧ for fine), scroll, or
// double-click to toggle between off and 0 dB.
// `moved(db, done)`: done is true when the change ends (commit it).
Item {
    id: knob
    property real db: -60
    property color tint: Theme.accent
    property string label: ""
    signal moved(real db, bool done)
    property real liveDb: -60
    readonly property real shownDb: area.pressed ? liveDb : db

    implicitWidth: 22
    implicitHeight: 22
    readonly property real minDb: -60
    readonly property real maxDb: 6
    readonly property real pos: shownDb <= minDb ? 0 : Math.pow((shownDb - minDb) / (maxDb - minDb), 2)
    function dbOf(p) { return p <= 0.0005 ? minDb : minDb + Math.sqrt(Math.min(1, p)) * (maxDb - minDb) }
    function emit(d, done) {
        const v = Math.round(Math.max(minDb, Math.min(maxDb, d)) * 10) / 10
        liveDb = v
        moved(v, done)
    }

    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
            strokeColor: Theme.lineStrong
            strokeWidth: 2.5
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            PathAngleArc { centerX: 11; centerY: 11; radiusX: 8.5; radiusY: 8.5; startAngle: 135; sweepAngle: 270 }
        }
        ShapePath {
            strokeColor: knob.tint
            strokeWidth: 2.5
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            PathAngleArc { centerX: 11; centerY: 11; radiusX: 8.5; radiusY: 8.5; startAngle: 135; sweepAngle: 270 * knob.pos }
        }
    }
    Rectangle {
        anchors.centerIn: parent
        width: 4; height: 4; radius: 2
        color: knob.pos > 0 ? knob.tint : Theme.textFaint
    }

    MouseArea {
        id: area
        anchors.fill: parent
        hoverEnabled: true
        preventStealing: true
        cursorShape: Qt.SizeVerCursor
        property real startY
        property real startPos
        onPressed: m => { knob.liveDb = knob.db; startY = m.y; startPos = knob.pos }
        onPositionChanged: m => {
            if (!pressed) return
            const scale = (m.modifiers & Qt.ShiftModifier) ? 0.2 : 1
            knob.emit(knob.dbOf(startPos + (startY - m.y) / 100 * scale), false)
        }
        onReleased: knob.emit(knob.liveDb, true)
        onDoubleClicked: knob.emit(knob.db > knob.minDb ? knob.minDb : 0, true)
        onWheel: w => knob.emit(knob.db + (w.angleDelta.y !== 0 ? w.angleDelta.y / 120 : w.pixelDelta.y / 30), true)
        DToolTip {
            visible: parent.containsMouse || parent.pressed
            delay: parent.pressed ? 0 : 450
            text: "Send to " + knob.label + ": "
                  + (knob.shownDb <= knob.minDb ? "off" : (knob.shownDb > 0 ? "+" : "") + knob.shownDb.toFixed(1) + " dB")
                  + (parent.pressed ? "" : " · double-click for 0 dB / off")
        }
    }
}
