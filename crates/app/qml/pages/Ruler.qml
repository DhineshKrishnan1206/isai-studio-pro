import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Isai

// The bar ruler: markers on top, then (when shown) the signature and tempo
// lanes, then the cycle strip, then bar numbers.
//   • Signature lane: click a signature to change or remove it; double-click
//     the lane to add one at that bar.
//   • Tempo lane: drag a point up or down to change its tempo, sideways to
//     move it; double-click the lane to add a change; right-click to remove.
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
    /// The signature and tempo lanes (Logic's global tracks), shown on request.
    property bool globalShown: false
    readonly property int sigHeight: globalShown ? 20 : 0
    readonly property int tempoHeight: globalShown ? 44 : 0
    /// Top tier: the cycle strip. Drag it to draw a loop, as Logic's ribbon.
    readonly property int cycleY: 0
    readonly property int stripHeight: 14
    /// Bottom tier: the time scale with bars, beats and divisions.
    readonly property int scaleY: stripHeight
    readonly property int scaleHeight: 26
    /// Markers sit under the time scale, then the global lanes.
    readonly property int markerY: scaleY + scaleHeight
    readonly property int sigY: markerY + markerHeight
    readonly property int tempoY: sigY + sigHeight
    implicitHeight: 58 + sigHeight + tempoHeight
    readonly property var markers: {
        try { return JSON.parse(app.markersJson || "[]") } catch (e) { return [] }
    }
    height: implicitHeight

    /// The bars in view (bars change length with the time signature).
    readonly property var visibleBars: TimeMap.barsBetween(beatAt(viewStart), beatAt(viewStart + viewWidth), 800)
    readonly property real barPx: TimeMap.barLenAt(beatAt(viewStart)) * pxPerBeat
    /// Label every Nth bar so numbers never collide.
    readonly property int labelEvery: barPx >= 34 ? 1 : barPx >= 17 ? 2 : barPx >= 9 ? 4 : barPx >= 4 ? 8 : 16

    function beatAt(x) { return Math.max(0, x / pxPerBeat) }

    // ------------------------------------------------------------ markers
    MouseArea {
        id: markerLane
        y: ruler.markerY
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
            y: ruler.markerY
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
            id: barItem
            required property var modelData
            readonly property real beatPx: modelData.unit * ruler.pxPerBeat
            // Beat ticks only when there is room; finer divisions when zoomed far in.
            readonly property int beatTicks: beatPx >= 8 ? modelData.num : 0
            readonly property int divisions: beatPx >= 60 ? 4 : beatPx >= 30 ? 2 : 1
            x: modelData.beat * ruler.pxPerBeat
            y: ruler.scaleY
            height: ruler.scaleHeight
            Rectangle { anchors.bottom: parent.bottom; width: 1; height: 10; color: Theme.lineStrong }
            DText {
                visible: (barItem.modelData.bar - 1) % ruler.labelEvery === 0
                x: 4
                anchors.bottom: parent.bottom
                anchors.bottomMargin: 7
                text: barItem.modelData.bar
                font.family: Theme.mono
                font.pixelSize: Theme.textXs
                color: Theme.textDim
            }
            Repeater {
                model: Math.max(0, barItem.beatTicks * barItem.divisions - 1)
                delegate: Rectangle {
                    required property int index
                    readonly property bool onBeat: (index + 1) % barItem.divisions === 0
                    x: (index + 1) * barItem.beatPx / barItem.divisions
                    anchors.bottom: parent.bottom
                    width: 1
                    height: onBeat ? 5 : 3
                    color: onBeat ? Theme.lineStrong : Theme.line
                }
            }
        }
    }

    // ------------------------------------------------- signature lane
    Rectangle {
        visible: ruler.globalShown
        x: ruler.viewStart; y: ruler.sigY
        width: ruler.viewWidth; height: ruler.sigHeight
        color: Theme.tint(Theme.sunken, 0.35)
        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.line }
    }
    MouseArea {
        visible: ruler.globalShown
        y: ruler.sigY
        width: parent.width; height: ruler.sigHeight
        hoverEnabled: true
        // Double-click: a new signature from the next bar line.
        onDoubleClicked: m => {
            const beat = ruler.beatAt(m.x)
            const bar = TimeMap.isBarLine(beat) ? TimeMap.barAt(beat) : TimeMap.barAt(beat) + 1
            const s = TimeMap.sectionAt(beat)
            sigPopup.openFor(bar, s[2], s[3], m.x)
        }
        DToolTip { visible: parent.containsMouse && TimeMap.sigs.length < 2; text: "Time signatures · double-click to change it from a bar on" }
    }
    Repeater {
        model: ruler.globalShown ? TimeMap.sigs : []
        delegate: AbstractButton {
            id: sigChip
            required property var modelData
            x: modelData[0] * ruler.pxPerBeat
            y: ruler.sigY + 2
            height: ruler.sigHeight - 4
            width: sigText.implicitWidth + 12
            visible: x + width >= ruler.viewStart - 40 && x <= ruler.viewStart + ruler.viewWidth + 40
            hoverEnabled: true
            onClicked: sigPopup.openFor(modelData[1], modelData[2], modelData[3], x)
            background: Rectangle {
                radius: 3
                color: sigChip.hovered ? Theme.hover : Theme.raised
                border.color: Theme.lineStrong
                Rectangle { width: 2; height: parent.height; color: Theme.accent }
            }
            contentItem: DText {
                id: sigText
                leftPadding: 4
                text: sigChip.modelData[2] + "/" + sigChip.modelData[3]
                font.family: Theme.mono
                font.pixelSize: 10
                verticalAlignment: Text.AlignVCenter
            }
            DToolTip { visible: sigChip.hovered; text: "From bar " + sigChip.modelData[1] + " · click to change" }
        }
    }
    Popup {
        id: sigPopup
        property int bar: 1
        property int num: 4
        property int den: 4
        readonly property bool exists: TimeMap.sigs.some(s => s[1] === bar)
        function openFor(b, n, d, px) { bar = b; num = n; den = d; x = Math.max(ruler.viewStart, px - 20); y = ruler.sigY + ruler.sigHeight; open() }
        padding: 12
        background: GlassPanel { backdrop: Theme.backdrop; radius: Theme.radiusLg; clarity: 0.3 }
        contentItem: ColumnLayout {
            spacing: 10
            DText { text: sigPopup.bar === 1 ? "Time signature at the start" : "Time signature from bar " + sigPopup.bar; strong: true; font.pixelSize: Theme.textSm }
            RowLayout {
                spacing: 6
                DButton { text: "−"; implicitWidth: 30; onClicked: sigPopup.num = Math.max(1, sigPopup.num - 1) }
                DText { text: sigPopup.num; font.family: Theme.mono; font.pixelSize: Theme.textLg; horizontalAlignment: Text.AlignHCenter; Layout.preferredWidth: 28 }
                DButton { text: "+"; implicitWidth: 30; onClicked: sigPopup.num = Math.min(32, sigPopup.num + 1) }
                DText { text: "/"; dim: true }
                Segmented {
                    minItemWidth: 34
                    options: [2, 4, 8, 16].map(d => ({ value: d, label: "" + d }))
                    value: sigPopup.den
                    onActivated: v => sigPopup.den = v
                }
            }
            RowLayout {
                spacing: 8
                DButton {
                    visible: sigPopup.exists && sigPopup.bar > 1
                    variant: "danger"
                    text: "Remove"
                    onClicked: { ruler.app.removeSignatureChange(sigPopup.bar); sigPopup.close() }
                }
                Item { Layout.fillWidth: true }
                DButton { variant: "ghost"; text: "Cancel"; onClicked: sigPopup.close() }
                DButton {
                    variant: "primary"
                    text: "Apply"
                    onClicked: { ruler.app.addSignatureChange(sigPopup.bar, sigPopup.num, sigPopup.den); sigPopup.close() }
                }
            }
        }
    }

    // ------------------------------------------------- tempo lane
    /// The tempo point being dragged: { index, beat, bpm }, or null.
    property var tempoDrag: null
    readonly property var tempoPts: TimeMap.tempos.map((t, i) => tempoDrag && tempoDrag.index === i ? [tempoDrag.beat, tempoDrag.bpm] : t)
    readonly property real tempoLo: Math.min(...tempoPts.map(t => t[1])) - 15
    readonly property real tempoHi: Math.max(tempoLo + 40, Math.max(...tempoPts.map(t => t[1])) + 15)
    function tempoYOf(bpm) { return tempoY + 6 + (1 - (bpm - tempoLo) / (tempoHi - tempoLo)) * (tempoHeight - 12) }

    Rectangle {
        visible: ruler.globalShown
        x: ruler.viewStart; y: ruler.tempoY
        width: ruler.viewWidth; height: ruler.tempoHeight
        color: Theme.tint(Theme.sunken, 0.5)
        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.line }
        DText { anchors.right: parent.right; anchors.rightMargin: 8; y: 3; text: "TEMPO"; faint: true; font.pixelSize: 9; font.letterSpacing: 1 }
    }
    MouseArea {
        visible: ruler.globalShown
        y: ruler.tempoY
        width: parent.width; height: ruler.tempoHeight
        hoverEnabled: true
        onDoubleClicked: m => { const b = ruler.beatAt(m.x); ruler.app.addTempoChange(b, TimeMap.bpmAt(b)) }
        DToolTip { visible: parent.containsMouse && TimeMap.tempos.length < 2; text: "Tempo · double-click to add a change, then drag it up or down" }
    }
    // The tempo line: flat while the tempo holds, a step at each change.
    Repeater {
        model: ruler.globalShown ? ruler.tempoPts : []
        delegate: Item {
            required property var modelData
            required property int index
            readonly property var next: index + 1 < ruler.tempoPts.length ? ruler.tempoPts[index + 1] : null
            readonly property real x0: modelData[0] * ruler.pxPerBeat
            readonly property real x1: next ? next[0] * ruler.pxPerBeat : ruler.width
            readonly property real yy: ruler.tempoYOf(modelData[1])
            Rectangle { x: parent.x0; y: parent.yy; width: Math.max(1, parent.x1 - parent.x0); height: 2; color: Theme.accent }
            Rectangle {
                visible: !!parent.next
                x: parent.x1
                y: Math.min(parent.yy, parent.next ? ruler.tempoYOf(parent.next[1]) : 0)
                width: 2
                height: parent.next ? Math.abs(ruler.tempoYOf(parent.next[1]) - parent.yy) + 2 : 0
                color: Theme.accent
            }
        }
    }
    Repeater {
        model: ruler.globalShown ? ruler.tempoPts : []
        delegate: Item {
            id: tp
            required property var modelData
            required property int index
            readonly property var original: TimeMap.tempos[index] || modelData
            x: modelData[0] * ruler.pxPerBeat - 6
            y: ruler.tempoYOf(modelData[1]) - 5
            width: 12; height: 12
            z: 3
            Rectangle {
                anchors.centerIn: parent
                width: 9; height: 9; radius: 4.5
                color: tpArea.containsMouse || tpArea.pressed ? Theme.accent : Theme.panel
                border.color: Theme.accent
                border.width: 2
            }
            DText {
                x: 12; y: -2
                text: tp.modelData[1].toFixed(1)
                font.family: Theme.mono
                font.pixelSize: 9
                color: Theme.text
            }
            MouseArea {
                id: tpArea
                anchors.fill: parent
                anchors.margins: -3
                hoverEnabled: true
                preventStealing: true
                acceptedButtons: Qt.LeftButton | Qt.RightButton
                cursorShape: tp.index === 0 ? Qt.SizeVerCursor : Qt.SizeAllCursor
                property real pressX
                property real pressY
                onPressed: m => {
                    if (m.button === Qt.RightButton) {
                        if (tp.index > 0) ruler.app.removeTempoChange(tp.original[0])
                        return
                    }
                    const p = mapToItem(ruler, m.x, m.y)
                    pressX = p.x; pressY = p.y
                }
                onPositionChanged: m => {
                    if (!pressed || !(pressedButtons & Qt.LeftButton)) return
                    const p = mapToItem(ruler, m.x, m.y)
                    // Half a BPM per pixel up or down; the first point stays at the start.
                    const bpm = Math.max(20, Math.min(400, Math.round((tp.original[1] + (pressY - p.y) * 0.5) * 10) / 10))
                    let beat = tp.original[0]
                    if (tp.index > 0) {
                        const prev = TimeMap.tempos[tp.index - 1][0], nxt = TimeMap.tempos[tp.index + 1]
                        beat = ruler.app.snapBeats(ruler.beatAt(p.x))
                        beat = Math.max(prev + 0.25, nxt ? Math.min(nxt[0] - 0.25, beat) : beat)
                    }
                    ruler.tempoDrag = { index: tp.index, beat: beat, bpm: bpm }
                }
                onReleased: m => {
                    if (m.button === Qt.RightButton || !ruler.tempoDrag) return
                    const d = ruler.tempoDrag
                    ruler.tempoDrag = null
                    if (tp.index === 0) ruler.app.addTempoChange(0, d.bpm)
                    else ruler.app.moveTempoChange(tp.original[0], d.beat, d.bpm)
                }
            }
            DToolTip { visible: tpArea.containsMouse && !tpArea.pressed; text: tp.index === 0 ? "Opening tempo · drag up or down" : "Drag to change the tempo or move it · right-click to remove" }
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
        y: ruler.scaleY
        width: parent.width
        height: ruler.scaleHeight
        cursorShape: Qt.PointingHandCursor
        onPressed: m => ruler.app.seekBeats(ruler.app.snapBeats(ruler.beatAt(m.x)))
        onPositionChanged: m => { if (pressed) ruler.app.seekBeats(ruler.app.snapBeats(ruler.beatAt(m.x))) }
    }
}
