import QtQuick
import Isai

// The bar ruler: markers on top, then the cycle strip, then bar numbers.
//   • Markers: double-click the lane (or press M) to add one. Click a marker
//     to jump there, drag it to move it, double-click to rename it,
//     right-click to cycle its section or delete it.
//   • Cycle strip: drag to draw a cycle region; drag it to move; drag its
//     edges to resize; click it to switch cycle on or off.
//   • Bar numbers: click or drag to move the playhead.
// Everything snaps to the grid when snap is on.
//
// The ruler is as wide as the whole timeline and scrolls with it; only the
// bars inside the visible window (`viewStart`, `viewWidth`) are drawn.
Item {
    id: ruler
    property var app
    /// Pixels per beat; shared with the timeline below.
    property real pxPerBeat: 10
    property real viewStart: 0
    property real viewWidth: width

    readonly property int markerHeight: 18
    /// Where the cycle strip starts (under the markers).
    readonly property int cycleY: markerHeight
    readonly property int stripHeight: 14
    implicitHeight: 58
    readonly property var markers: {
        try { return JSON.parse(app.markersJson || "[]") } catch (e) { return [] }
    }
    height: implicitHeight

    readonly property real barBeats: Math.max(1, app.timeSigNum)
    readonly property real barPx: barBeats * pxPerBeat
    /// Label every Nth bar so numbers never collide.
    readonly property int labelEvery: barPx >= 34 ? 1 : barPx >= 17 ? 2 : barPx >= 9 ? 4 : barPx >= 4 ? 8 : 16
    readonly property int firstBar: Math.max(0, Math.floor(viewStart / barPx))
    readonly property int visibleBars: Math.ceil(viewWidth / barPx) + 2
    /// Beat ticks only when there is room; finer divisions when zoomed far in.
    readonly property int beatTicks: pxPerBeat >= 8 ? barBeats : 0
    readonly property int divisions: pxPerBeat >= 60 ? 4 : pxPerBeat >= 30 ? 2 : 1

    function beatAt(x) { return Math.max(0, x / pxPerBeat) }

    // ------------------------------------------------------------ markers
    MouseArea {
        id: markerLane
        width: parent.width
        height: ruler.markerHeight
        acceptedButtons: Qt.LeftButton
        onDoubleClicked: m => ruler.app.addMarker(ruler.beatAt(m.x))
        DToolTip { visible: markerLane.containsMouse && ruler.markers.length === 0; text: "Double-click to add a marker (M adds one at the playhead)" }
        hoverEnabled: true
    }
    Repeater {
        model: ruler.markers
        delegate: Item {
            id: flag
            required property var modelData
            readonly property real beat: dragging ? dragBeat : modelData.beat
            property bool dragging: false
            property real dragBeat: 0
            x: beat * ruler.pxPerBeat
            height: ruler.markerHeight
            width: Math.max(18, label.implicitWidth + 14)
            visible: x + width >= ruler.viewStart - 50 && x <= ruler.viewStart + ruler.viewWidth + 50
            z: dragging ? 5 : 1
            Rectangle {
                anchors.fill: parent
                anchors.topMargin: 2
                anchors.bottomMargin: 1
                radius: 4
                color: flagArea.containsMouse || flag.dragging ? Qt.lighter(Theme.marker, 1.15) : Theme.marker
                Rectangle { width: 2; height: parent.height; color: Qt.darker(Theme.marker, 1.5) }
                DText {
                    id: label
                    x: 7
                    anchors.verticalCenter: parent.verticalCenter
                    visible: !rename.visible
                    text: flag.modelData.name
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                    color: "#1A1205"
                }
                TextInput {
                    id: rename
                    visible: false
                    x: 7
                    width: Math.max(60, contentWidth + 4)
                    anchors.verticalCenter: parent.verticalCenter
                    font.family: Theme.font
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                    color: "#1A1205"
                    selectionColor: "#FFFFFF"
                    onAccepted: { const id = flag.modelData.id, name = text; visible = false; ruler.app.renameMarker(id, name) }
                    onActiveFocusChanged: if (!activeFocus) visible = false
                    Keys.onEscapePressed: visible = false
                }
            }
            MouseArea {
                id: flagArea
                anchors.fill: parent
                hoverEnabled: true
                preventStealing: true
                acceptedButtons: Qt.LeftButton | Qt.RightButton
                cursorShape: flag.dragging ? Qt.ClosedHandCursor : Qt.PointingHandCursor
                property real pressX
                property real grab
                onPressed: m => {
                    if (m.button === Qt.RightButton) { markerMenu.target = flag.modelData; markerMenu.x = flag.x + m.x; markerMenu.y = ruler.markerHeight; markerMenu.open(); return }
                    pressX = mapToItem(ruler, m.x, 0).x
                    grab = pressX - flag.x
                }
                onPositionChanged: m => {
                    if (!pressed || !(pressedButtons & Qt.LeftButton)) return
                    const x = mapToItem(ruler, m.x, 0).x
                    if (!flag.dragging && Math.abs(x - pressX) < 4) return
                    flag.dragging = true
                    flag.dragBeat = ruler.app.snapBeats(ruler.beatAt(x - grab))
                }
                onReleased: m => {
                    if (m.button === Qt.RightButton) return
                    const id = flag.modelData.id, to = flag.dragBeat, moved = flag.dragging
                    flag.dragging = false
                    if (moved) ruler.app.moveMarker(id, to)
                    else ruler.app.seekBeats(flag.modelData.beat)
                }
                onDoubleClicked: { rename.text = flag.modelData.name; rename.visible = true; rename.forceActiveFocus(); rename.selectAll() }
            }
            DToolTip { visible: flagArea.containsMouse && !flag.dragging; text: flag.modelData.name + " · click to go here, drag to move, double-click to rename, right-click for more" }
        }
    }
    MenuPopup {
        id: markerMenu
        property var target: null
        width: 200
        items: [
            { label: "Go Here", action: () => ruler.app.seekBeats(markerMenu.target.beat) },
            { label: "Cycle This Section", action: () => ruler.app.cycleMarker(markerMenu.target.id) },
            { separator: true },
            { label: "Delete Marker", danger: true, action: () => ruler.app.removeMarker(markerMenu.target.id) }
        ]
    }

    // Cycle strip background
    Rectangle {
        x: ruler.viewStart
        y: ruler.cycleY
        width: ruler.viewWidth
        height: ruler.stripHeight
        color: Theme.tint(Theme.sunken, 0.6)
    }

    // Bar and beat ticks (visible range only)
    Repeater {
        model: ruler.visibleBars
        delegate: Item {
            required property int index
            readonly property int bar: ruler.firstBar + index
            x: bar * ruler.barPx
            y: ruler.cycleY + ruler.stripHeight
            height: ruler.height - ruler.cycleY - ruler.stripHeight
            Rectangle { anchors.bottom: parent.bottom; width: 1; height: 10; color: Theme.lineStrong }
            DText {
                visible: parent.bar % ruler.labelEvery === 0
                x: 4
                anchors.bottom: parent.bottom
                anchors.bottomMargin: 7
                text: parent.bar + 1
                font.family: Theme.mono
                font.pixelSize: Theme.textXs
                color: Theme.textDim
            }
            Repeater {
                model: Math.max(0, ruler.beatTicks * ruler.divisions - 1)
                delegate: Rectangle {
                    required property int index
                    readonly property bool onBeat: (index + 1) % ruler.divisions === 0
                    x: (index + 1) * ruler.pxPerBeat / ruler.divisions
                    anchors.bottom: parent.bottom
                    width: 1
                    height: onBeat ? 5 : 3
                    color: onBeat ? Theme.lineStrong : Theme.line
                }
            }
        }
    }

    Rectangle { x: ruler.viewStart; anchors.bottom: parent.bottom; width: ruler.viewWidth; height: 1; color: Theme.line }

    // Cycle region (shown dimmed when cycle is off)
    Rectangle {
        readonly property real start: drag.active ? drag.previewStart : ruler.app.cycleStart
        readonly property real end: drag.active ? drag.previewEnd : ruler.app.cycleEnd
        x: start * ruler.pxPerBeat
        width: Math.max(2, (end - start) * ruler.pxPerBeat)
        y: ruler.cycleY + 1
        height: ruler.stripHeight - 2
        radius: 3
        color: ruler.app.cycle || drag.active ? Theme.loopOn : Theme.tint(Theme.textFaint, 0.45)
    }

    MouseArea {
        id: drag
        y: ruler.cycleY
        width: parent.width
        height: ruler.stripHeight
        hoverEnabled: true

        // "new", "move", "start" or "end"
        property string mode: ""
        property bool active: false
        property real anchorBeat: 0
        property real grabOffset: 0
        property real previewStart: 0
        property real previewEnd: 0
        property real pressX: 0

        function hit(x) {
            const s = ruler.app.cycleStart * ruler.pxPerBeat
            const e = ruler.app.cycleEnd * ruler.pxPerBeat
            if (Math.abs(x - s) <= 6) return "start"
            if (Math.abs(x - e) <= 6) return "end"
            if (x > s && x < e) return "move"
            return "new"
        }

        cursorShape: {
            const h = hit(mouseX)
            return h === "start" || h === "end" ? Qt.SizeHorCursor : h === "move" ? Qt.OpenHandCursor : Qt.CrossCursor
        }

        onPressed: m => {
            mode = hit(m.x)
            pressX = m.x
            anchorBeat = ruler.app.snapBeats(ruler.beatAt(m.x))
            grabOffset = ruler.beatAt(m.x) - ruler.app.cycleStart
            previewStart = ruler.app.cycleStart
            previewEnd = ruler.app.cycleEnd
        }
        onPositionChanged: m => {
            if (!pressed) return
            if (!active && Math.abs(m.x - pressX) < 3) return
            active = true
            const b = ruler.app.snapBeats(ruler.beatAt(m.x))
            const len = ruler.app.cycleEnd - ruler.app.cycleStart
            if (mode === "new") {
                previewStart = Math.min(anchorBeat, b)
                previewEnd = Math.max(anchorBeat, b)
            } else if (mode === "move") {
                previewStart = ruler.app.snapBeats(Math.max(0, ruler.beatAt(m.x) - grabOffset))
                previewEnd = previewStart + len
            } else if (mode === "start") {
                previewStart = Math.min(b, ruler.app.cycleEnd)
            } else if (mode === "end") {
                previewEnd = Math.max(b, ruler.app.cycleStart)
            }
        }
        onReleased: {
            if (active) {
                ruler.app.setCycleRange(previewStart, previewEnd)
            } else if (mode !== "new") {
                ruler.app.toggleCycle()
            }
            active = false
        }
    }

    // Bar numbers: move the playhead.
    MouseArea {
        y: ruler.cycleY + ruler.stripHeight
        width: parent.width
        height: ruler.height - ruler.cycleY - ruler.stripHeight
        cursorShape: Qt.PointingHandCursor
        onPressed: m => ruler.app.seekBeats(ruler.app.snapBeats(ruler.beatAt(m.x)))
        onPositionChanged: m => { if (pressed) ruler.app.seekBeats(ruler.app.snapBeats(ruler.beatAt(m.x))) }
    }
}
