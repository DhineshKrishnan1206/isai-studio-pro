import QtQuick
import QtQuick.Shapes
import Isai

// One automation lane under a track: a line through points over time.
//   • Click empty space to add a point there (snapped to the grid).
//   • Drag a point to move it; double-click (or ⌥-click) it to delete it.
// While playing, the track follows the line.
Item {
    id: lane
    property var app
    property var track
    /// The target being shown (an entry of `track.autoTargets`).
    property var target
    property real pxPerBeat: 10
    property color tint: Theme.accent
    /// Visible part of the timeline (only the points there get dots).
    property real viewStart: 0
    property real viewWidth: width

    readonly property var points: target ? target.points : []
    readonly property real usable: height - 12

    // A point being dragged: its index and where it is now (beat, value).
    property int dragIndex: -1
    property real dragBeat: 0
    property real dragValue: 0

    readonly property var shownPoints: {
        if (dragIndex < 0) return points
        const p = points.slice()
        p[dragIndex] = [dragBeat, dragValue]
        return p.sort((a, b) => a[0] - b[0])
    }

    function yOf(v) { return 6 + (1 - Auto.norm(target, v)) * usable }
    function valueAtY(y) { return Auto.denorm(target, 1 - (y - 6) / usable) }

    Rectangle {
        anchors.fill: parent
        radius: Theme.radiusSm
        color: Theme.tint(lane.tint, 0.06)
        border.color: Theme.tint(lane.tint, 0.18)
    }

    // The line: flat from the start to the first point, through every point,
    // flat after the last. Dashed at the track's own value when there are none.
    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer
        visible: !!lane.target
        ShapePath {
            strokeColor: lane.points.length > 0 ? lane.tint : Theme.tint(lane.tint, 0.5)
            strokeWidth: 2
            strokeStyle: lane.points.length > 0 ? ShapePath.SolidLine : ShapePath.DashLine
            dashPattern: [4, 4]
            fillColor: "transparent"
            PathPolyline {
                path: {
                    const pts = lane.shownPoints
                    if (!lane.target) return []
                    if (pts.length === 0) {
                        const y = lane.yOf(lane.target.value)
                        return [Qt.point(0, y), Qt.point(lane.width, y)]
                    }
                    const out = [Qt.point(0, lane.yOf(pts[0][1]))]
                    for (const [b, v] of pts) out.push(Qt.point(b * lane.pxPerBeat, lane.yOf(v)))
                    out.push(Qt.point(lane.width, lane.yOf(pts[pts.length - 1][1])))
                    return out
                }
            }
        }
    }

    DText {
        visible: !!lane.target && lane.points.length === 0
        x: lane.viewStart + 10
        anchors.verticalCenter: parent.verticalCenter
        text: "Click to add points: " + (lane.target ? lane.target.label : "")
        faint: true
        font.pixelSize: Theme.textXs
    }

    // Empty space: add a point.
    MouseArea {
        anchors.fill: parent
        preventStealing: true
        cursorShape: Qt.CrossCursor
        onClicked: m => {
            if (!lane.target) return
            const beat = lane.app.snapBeats(Math.max(0, m.x / lane.pxPerBeat))
            lane.app.addAutoPoint(lane.track.id, lane.target.key, beat, lane.valueAtY(m.y))
        }
    }

    // Points (only those near the visible part of the timeline).
    Repeater {
        model: lane.points.length
        delegate: Item {
            id: dot
            required property int index
            readonly property var point: lane.points[index]
            readonly property bool dragging: lane.dragIndex === index
            readonly property real beat: dragging ? lane.dragBeat : point[0]
            readonly property real value: dragging ? lane.dragValue : point[1]
            visible: x + width >= lane.viewStart - 20 && x <= lane.viewStart + lane.viewWidth + 20
            x: beat * lane.pxPerBeat - width / 2
            y: lane.yOf(value) - height / 2
            width: 18; height: 18
            z: dragging ? 3 : 2

            Rectangle {
                anchors.centerIn: parent
                width: grab.containsMouse || dot.dragging ? 11 : 8
                height: width
                radius: width / 2
                color: Theme.dark ? "#FFFFFF" : Theme.surface
                border.color: lane.tint
                border.width: 2
            }

            MouseArea {
                id: grab
                anchors.fill: parent
                hoverEnabled: true
                preventStealing: true
                cursorShape: Qt.SizeAllCursor
                property real pressX
                property real pressY
                onPressed: m => {
                    if (m.modifiers & Qt.AltModifier) {
                        lane.app.removeAutoPoint(lane.track.id, lane.target.key, dot.index)
                        return
                    }
                    const p = mapToItem(lane, m.x, m.y)
                    pressX = p.x; pressY = p.y
                    lane.dragBeat = dot.point[0]
                    lane.dragValue = dot.point[1]
                }
                onPositionChanged: m => {
                    if (!pressed) return
                    const p = mapToItem(lane, m.x, m.y)
                    if (lane.dragIndex < 0 && Math.abs(p.x - pressX) < 3 && Math.abs(p.y - pressY) < 3) return
                    lane.dragIndex = dot.index
                    lane.dragBeat = lane.app.snapBeats(Math.max(0, p.x / lane.pxPerBeat))
                    lane.dragValue = lane.valueAtY(Math.max(6, Math.min(lane.height - 6, p.y)))
                }
                onReleased: {
                    // Tidy up before the edit, which can rebuild this lane.
                    const app = lane.app, track = lane.track.id, key = lane.target.key
                    const move = lane.dragIndex === dot.index, i = dot.index, beat = lane.dragBeat, value = lane.dragValue
                    lane.dragIndex = -1
                    if (move) app.moveAutoPoint(track, key, i, beat, value)
                }
                onDoubleClicked: lane.app.removeAutoPoint(lane.track.id, lane.target.key, dot.index)
            }

            DToolTip {
                visible: grab.containsMouse || dot.dragging
                delay: 0
                text: Auto.format(lane.target, dot.value) + (dot.dragging ? "" : " · drag to move, double-click to delete")
            }
        }
    }
}
