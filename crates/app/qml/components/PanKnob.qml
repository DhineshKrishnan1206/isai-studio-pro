import QtQuick
import QtQuick.Shapes
import Isai

// A small rotary pan knob. Drag up/down to turn, double-click to centre.
// `moved(value, done)`: done is true when the drag ends (commit the change).
Item {
    id: knob
    property real value: 0          // -1 .. 1
    property color tint: Theme.accent
    signal moved(real value, bool done)
    property real liveValue: 0
    readonly property real shownValue: area.pressed ? liveValue : value

    implicitWidth: 26
    implicitHeight: 26
    readonly property real angle: shownValue * 135    // degrees from the top

    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer
        // Track
        ShapePath {
            strokeColor: Theme.lineStrong
            strokeWidth: 2.5
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            PathAngleArc { centerX: 13; centerY: 13; radiusX: 10; radiusY: 10; startAngle: 135; sweepAngle: 270 }
        }
        // Value, drawn from the centre outwards
        ShapePath {
            strokeColor: knob.tint
            strokeWidth: 2.5
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            PathAngleArc { centerX: 13; centerY: 13; radiusX: 10; radiusY: 10; startAngle: -90; sweepAngle: knob.angle }
        }
    }
    // Pointer
    Rectangle {
        x: 12; y: 5
        width: 2; height: 7; radius: 1
        color: Theme.text
        transform: Rotation { origin.x: 1; origin.y: 8; angle: knob.angle }
    }

    MouseArea {
        id: area
        anchors.fill: parent
        hoverEnabled: true
        preventStealing: true
        cursorShape: Qt.SizeVerCursor
        property real startY
        property real startValue
        onPressed: m => { startY = m.y; startValue = knob.value; knob.liveValue = knob.value }
        onPositionChanged: m => {
            if (!pressed) return
            const v = Math.max(-1, Math.min(1, startValue + (startY - m.y) / 80))
            knob.liveValue = Math.round(v * 100) / 100
            knob.moved(knob.liveValue, false)
        }
        onReleased: knob.moved(knob.liveValue, true)
        onDoubleClicked: knob.moved(0, true)
        DToolTip {
            visible: parent.containsMouse && !parent.pressed
            text: "Pan " + (Math.abs(knob.shownValue) < 0.01 ? "centre"
                  : (knob.shownValue < 0 ? "L" : "R") + Math.round(Math.abs(knob.shownValue) * 100))
                  + " · double-click to centre"
        }
    }
}
