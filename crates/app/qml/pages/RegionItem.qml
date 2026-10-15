import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Shapes
import Isai

// An audio region on a track lane.
//   • Click to select, ⇧- or ⌘-click to add to the selection; drag to move it
//     (and the rest of the selection) in time or onto another track.
//   • Drag the left or right edge to trim.
//   • Drag the small handles at the top corners to set fade-in / fade-out.
//   • Drag the loop handle at the top-right corner to repeat the region;
//     double-click it to stop looping.
//   • ⌥-click to split at that point; right-click for more (gain, normalize,
//     reverse, join).
// Everything snaps to the grid when snap is on; changes apply on release.
Item {
    id: item
    property var app
    property var region          // one region from app.tracksJson
    property color tint
    property real pxPerBeat
    property real laneHeight
    property int trackIndex
    property var tracks          // all tracks, to find the drop target
    property var view            // the TrackArea, for moving the selection together
    property var rowTops: []     // where each track's row starts (rows can differ in height)
    function rowOffset(row) { return rowTops.length > row ? rowTops[row] - rowTops[trackIndex] : (row - trackIndex) * laneHeight }
    property bool selected: false
    /// The track is frozen: its regions aren't heard until it's unfrozen.
    property bool frozen: false
    readonly property bool isMidi: !!region.midi
    signal editRequested()
    /// "Split into Stems…" was chosen.
    signal stemsRequested()
    property int columnWidth: 2

    // Colours. Studio: solid header over a tinted body (Theme.tintedRegions).
    // Vivid: one solid colour with deeper ink. Midnight: solid colour, white ink.
    readonly property bool tinted: Theme.tintedRegions
    readonly property color headInk: Theme.vivid ? Qt.darker(tint, 2.6) : tinted ? Theme.regionHeaderText : "#FFFFFF"
    readonly property color bodyInk: Theme.vivid ? Qt.darker(tint, 2.2) : tinted ? Theme.regionInk(tint) : Qt.rgba(1, 1, 1, 0.85)

    // Live previews while dragging; they fall back to the region's values.
    property real previewStart: region.start
    property real previewEnd: region.start + region.beats
    property int previewTrack: trackIndex
    property real previewFadeIn: region.fadeIn
    property real previewFadeOut: region.fadeOut
    /// The loop's reach from the region's start (its own length when it doesn't loop).
    property real previewSpan: region.span
    property bool busy: false
    /// The left edge is being trimmed: the loop's end stays where it is.
    property bool trimmingLeft: false
    readonly property real natural: previewEnd - previewStart
    readonly property real bodyWidth: Math.max(4, natural * pxPerBeat)
    readonly property real spanRel: trimmingLeft ? previewSpan - (previewStart - region.start) : previewSpan
    readonly property bool looped: spanRel > natural + 1e-6

    // Another selected region is being dragged: follow it.
    readonly property bool following: selected && !busy && !!view && view.groupDragging
    x: (previewStart + (following ? view.groupDx : 0)) * pxPerBeat
    y: rowOffset(previewTrack + (following ? view.groupDy : 0))
    z: busy ? 10 : selected ? 2 : 1
    width: looped ? Math.max(bodyWidth, spanRel * pxPerBeat) : bodyWidth

    function reset() {
        busy = false
        previewStart = Qt.binding(() => region.start)
        previewEnd = Qt.binding(() => region.start + region.beats)
        previewTrack = Qt.binding(() => trackIndex)
        previewFadeIn = Qt.binding(() => region.fadeIn)
        previewFadeOut = Qt.binding(() => region.fadeOut)
        previewSpan = Qt.binding(() => region.span)
        trimmingLeft = false
    }

    // Loop passes after the first: lighter copies of the region.
    Repeater {
        model: item.looped && item.natural > 0 ? Math.max(0, Math.ceil(item.spanRel / item.natural - 1e-6) - 1) : 0
        delegate: Rectangle {
            id: pass
            required property int index
            readonly property real beats: Math.min(item.natural, item.spanRel - (index + 1) * item.natural)
            x: (index + 1) * item.bodyWidth
            width: Math.max(2, beats * item.pxPerBeat)
            height: item.height
            radius: Theme.radiusMd
            color: item.tinted ? Theme.tint(Theme.regionBody(item.tint), 0.75) : Theme.tint(item.tint, 0.5)
            border.width: 1
            border.color: Theme.tint(Qt.darker(item.tint, 1.4), 0.8)
            clip: true
            Row {
                x: 5; y: 2
                spacing: 3
                Icon { glyph: Icons.repeat; size: 10; color: item.tinted ? item.bodyInk : item.headInk; anchors.verticalCenter: parent.verticalCenter }
                DText {
                    text: item.region.name
                    font.pixelSize: Theme.textXs
                    color: item.tinted ? item.bodyInk : item.headInk
                    opacity: 0.85
                }
            }
            WaveformView {
                visible: item.region.state === "ready" && !item.isMidi
                anchors.fill: parent
                anchors.topMargin: 18
                anchors.margins: 3
                color: Theme.vivid ? Theme.tint(Qt.darker(item.tint, 1.9), 0.6) : item.tinted ? Theme.tint(item.bodyInk, 0.55) : Qt.rgba(0, 0, 0, 0.4)
                columnWidth: item.columnWidth
                source: item.region.source
                readonly property real secs: pass.beats * item.region.length / Math.max(1e-9, item.region.beats)
                offset: item.region.offset + (item.region.reverse ? item.region.length - secs : 0)
                length: secs
                gain: Math.pow(10, (item.region.gain || 0) / 20)
                reverse: !!item.region.reverse
            }
            Repeater {
                model: item.isMidi && item.region.midi.notes.length <= 300 ? item.region.midi.notes : []
                delegate: Rectangle {
                    required property var modelData
                    readonly property real rel: modelData[0] - item.region.midi.offset
                    visible: rel >= 0 && rel < pass.beats
                    x: rel * item.pxPerBeat
                    y: 20 + (noteView.high + (noteView.span - (noteView.high - noteView.low + 1)) / 2 - modelData[2]) / noteView.span * (pass.height - 26)
                    width: Math.max(2, modelData[1] * item.pxPerBeat - 1)
                    height: Math.max(2, Math.min(5, (pass.height - 24) / noteView.span))
                    radius: 1
                    color: Theme.tint(item.bodyInk, 0.6)
                }
            }
        }
    }

    Rectangle {
        id: body
        width: item.bodyWidth
        height: parent.height
        radius: Theme.radiusMd
        color: item.region.state === "missing" ? Theme.tint(Theme.danger, 0.25) : item.tinted ? Theme.regionBody(item.tint) : item.tint
        border.width: item.selected ? 2 : Theme.vivid ? 0 : 1
        border.color: item.selected ? (Theme.dark ? "#FFFFFF" : "#141416") : Qt.darker(item.tint, item.tinted ? 1.6 : 1.5)
        opacity: item.busy ? 0.88 : item.frozen ? 0.45 : 1
        clip: true

        Rectangle {
            id: title
            width: parent.width
            height: 18
            // Vivid: one solid colour, with the name and waveform in a deeper shade of it.
            color: Theme.vivid ? "transparent" : item.tinted ? item.tint : Qt.darker(item.tint, 1.25)
            // Gain and reverse badges.
            Row {
                id: badges
                anchors.right: parent.right; anchors.rightMargin: 5
                anchors.verticalCenter: parent.verticalCenter
                spacing: 4
                DText {
                    visible: Math.abs(item.region.gain || 0) > 0.05
                    text: ((item.region.gain || 0) > 0 ? "+" : "") + (item.region.gain || 0).toFixed(1) + " dB"
                    font.family: Theme.mono
                    font.pixelSize: 9
                    color: item.headInk
                }
                Icon {
                    visible: !!item.region.tempo
                    glyph: Icons.metronome
                    size: 10
                    color: item.headInk
                    anchors.verticalCenter: parent.verticalCenter
                }
                Icon {
                    visible: !!item.region.reverse
                    glyph: Icons.arrowLeft
                    size: 10
                    color: item.headInk
                    anchors.verticalCenter: parent.verticalCenter
                }
            }
            DText {
                anchors.left: parent.left; anchors.leftMargin: 6
                anchors.right: badges.left; anchors.rightMargin: 6
                anchors.verticalCenter: parent.verticalCenter
                text: item.region.name
                font.pixelSize: Theme.textXs
                font.weight: Font.DemiBold
                font.capitalization: Theme.vivid ? Font.AllUppercase : Font.MixedCase
                font.letterSpacing: Theme.vivid ? 0.4 : 0
                color: item.headInk
                elide: Text.ElideRight
            }
        }
        // MIDI: the notes, scaled to fit between the region's lowest and highest key.
        Item {
            id: noteView
            visible: item.isMidi
            anchors.top: title.bottom
            anchors.left: parent.left; anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.margins: 4
            clip: true
            readonly property var notes: item.isMidi ? item.region.midi.notes : []
            readonly property int low: notes.reduce((m, n) => Math.min(m, n[2]), 127)
            readonly property int high: notes.reduce((m, n) => Math.max(m, n[2]), 0)
            readonly property int span: Math.max(12, high - low + 1)
            readonly property real offset: item.isMidi ? item.region.midi.offset : 0
            Repeater {
                model: noteView.notes.length <= 600 ? noteView.notes : noteView.notes.slice(0, 600)
                delegate: Rectangle {
                    required property var modelData
                    readonly property real rel: modelData[0] - noteView.offset
                    visible: rel + modelData[1] > 0
                    x: rel * item.pxPerBeat
                    width: Math.max(2, modelData[1] * item.pxPerBeat - 1)
                    height: Math.max(2, Math.min(5, noteView.height / noteView.span))
                    y: (noteView.high + (noteView.span - (noteView.high - noteView.low + 1)) / 2 - modelData[2]) / noteView.span * (noteView.height - height)
                    radius: 1
                    color: item.bodyInk
                }
            }
            DText {
                visible: noteView.notes.length === 0
                anchors.centerIn: parent
                text: "Double-click to add notes"
                font.pixelSize: Theme.textXs
                color: item.bodyInk
                opacity: 0.8
            }
        }

        WaveformView {
            visible: item.region.state === "ready" && !item.isMidi
            anchors.top: title.bottom
            anchors.left: parent.left; anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.margins: 3
            color: Theme.vivid ? Theme.tint(Qt.darker(item.tint, 1.9), 0.9) : item.tinted ? item.bodyInk : Qt.rgba(0, 0, 0, 0.6)
            columnWidth: item.columnWidth
            source: item.region.source
            // While trimming, show the slice the trim would keep.
            offset: item.region.offset + (item.previewStart - item.region.start) * item.region.length / Math.max(1e-9, item.region.beats)
            length: (item.previewEnd - item.previewStart) * item.region.length / Math.max(1e-9, item.region.beats)
            gain: Math.pow(10, (item.region.gain || 0) / 20)
            reverse: !!item.region.reverse
            onWidthChanged: update()
            onHeightChanged: update()
        }

        // Fade curves: the shaded area is what the fade turns down.
        Shape {
            id: fades
            anchors.fill: parent
            anchors.topMargin: title.height
            visible: item.previewFadeIn > 0 || item.previewFadeOut > 0
            preferredRendererType: Shape.CurveRenderer
            readonly property real fi: item.previewFadeIn * item.pxPerBeat
            readonly property real fo: item.previewFadeOut * item.pxPerBeat
            readonly property real h: height
            ShapePath {
                strokeColor: "#FFFFFF"
                strokeWidth: 1.2
                fillColor: Qt.rgba(0, 0, 0, 0.28)
                startX: 0; startY: 0
                PathLine { x: 0; y: fades.h }
                PathQuad { x: fades.fi; y: 0; controlX: fades.fi * 0.15; controlY: fades.h * 0.1 }
                PathLine { x: 0; y: 0 }
            }
            ShapePath {
                strokeColor: "#FFFFFF"
                strokeWidth: 1.2
                fillColor: Qt.rgba(0, 0, 0, 0.28)
                startX: body.width; startY: 0
                PathLine { x: body.width; y: fades.h }
                PathQuad { x: body.width - fades.fo; y: 0; controlX: body.width - fades.fo * 0.15; controlY: fades.h * 0.1 }
                PathLine { x: body.width; y: 0 }
            }
        }

        Row {
            visible: item.region.state !== "ready"
            anchors.centerIn: parent
            anchors.verticalCenterOffset: 8
            spacing: 6
            Spinner { visible: item.region.state === "loading"; size: 14; color: "#FFFFFF" }
            DText {
                text: item.region.state === "loading" ? "Loading…" : "Audio file missing"
                font.pixelSize: Theme.textXs
                color: "#FFFFFF"
            }
        }
    }

    // ------------------------------------------------------- move / split
    MouseArea {
        id: moveArea
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        // Without this the timeline scrolls instead once the mouse moves a few pixels.
        preventStealing: true
        hoverEnabled: true
        cursorShape: item.busy ? Qt.ClosedHandCursor : Qt.OpenHandCursor
        property real pressX
        property real pressY
        property bool moving: false
        // Clicking one region of a bigger selection selects just it, unless it was dragged.
        property bool narrowOnRelease: false
        readonly property bool inGroup: item.selected && item.app.selectionCount > 1

        onPressed: m => {
            moving = false
            narrowOnRelease = false
            if (m.button === Qt.RightButton) {
                if (!item.selected) item.app.selectRegion(item.region.id)
                regionMenu.x = m.x; regionMenu.y = m.y; regionMenu.open()
                return
            }
            if (m.modifiers & (Qt.ShiftModifier | Qt.ControlModifier)) {
                item.app.toggleRegionSelection(item.region.id)
                return
            }
            if (m.modifiers & Qt.AltModifier) {
                item.app.selectRegion(item.region.id)
                item.app.splitRegion(item.region.id, item.app.snapBeats(item.region.start + m.x / item.pxPerBeat))
                return
            }
            if (inGroup) narrowOnRelease = true
            else item.app.selectRegion(item.region.id)
            const p = mapToItem(item.parent, m.x, m.y)
            pressX = p.x; pressY = p.y
            moving = true
        }
        onPositionChanged: m => {
            if (!pressed || !moving) return
            const p = mapToItem(item.parent, m.x, m.y)
            const dx = p.x - pressX, dy = p.y - pressY
            if (!item.busy && Math.abs(dx) < 4 && Math.abs(dy) < 4) return
            item.busy = true
            const len = item.region.beats
            let start = item.app.snapBeats(Math.max(0, item.region.start + dx / item.pxPerBeat))
            let row = item.view ? item.view.rowAt(item.rowTops[item.trackIndex] + item.laneHeight / 2 + dy)
                                : Math.max(0, Math.min(item.tracks.length - 1, item.trackIndex + Math.round(dy / item.laneHeight)))
            if (inGroup && item.view) {
                // Keep the whole selection inside the song and the track list.
                const b = item.view.selectionBounds
                const shift = Math.max(start - item.region.start, -b.minStart)
                const rows = Math.max(-b.top, Math.min(item.tracks.length - 1 - b.bottom, row - item.trackIndex))
                start = item.region.start + shift
                row = item.trackIndex + rows
                item.view.groupDx = shift
                item.view.groupDy = rows
                item.view.groupDragging = true
            }
            item.previewStart = start
            item.previewEnd = start + len
            item.previewTrack = row
        }
        onDoubleClicked: if (item.isMidi) item.editRequested()
        onReleased: {
            // Work out the edit, tidy up, then apply it: the edit republishes
            // the tracks, which can replace this item (e.g. moved to another track).
            const app = item.app, id = item.region.id
            const move = item.busy && (item.previewStart !== item.region.start || item.previewTrack !== item.trackIndex)
            const start = item.previewStart, track = item.tracks[item.previewTrack].id
            const select = !move && moving && narrowOnRelease
            moving = false
            narrowOnRelease = false
            if (item.view) {
                item.view.groupDragging = false
                item.view.groupDx = 0
                item.view.groupDy = 0
            }
            item.reset()
            if (move) app.moveRegion(id, start, track)
            else if (select) app.selectRegion(id)
        }
    }

    // ------------------------------------------------------------- trim
    component TrimEdge: MouseArea {
        id: edge
        property bool leftEdge: true
        width: 7
        height: parent.height - 18
        y: 18
        x: leftEdge ? 0 : item.bodyWidth - width
        cursorShape: Qt.SizeHorCursor
        hoverEnabled: true
        preventStealing: true
        onPressed: { item.app.selectRegion(item.region.id); item.trimmingLeft = leftEdge }
        onPositionChanged: m => {
            if (!pressed) return
            item.busy = true
            const beat = item.app.snapBeats(item.x / item.pxPerBeat + mapToItem(item, m.x, 0).x / item.pxPerBeat)
            if (leftEdge) item.previewStart = Math.min(beat, item.previewEnd - 0.01)
            else item.previewEnd = Math.max(beat, item.previewStart + 0.01)
        }
        onReleased: {
            // Reset first: trimming republishes the tracks, which can replace this item.
            const app = item.app, id = item.region.id, busy = item.busy
            const beat = leftEdge ? item.previewStart : item.previewEnd
            item.reset()
            if (busy) app.trimRegion(id, leftEdge, beat)
        }
        Rectangle {
            anchors.fill: parent
            visible: edge.containsMouse || edge.pressed
            color: Qt.rgba(1, 1, 1, 0.25)
        }
    }
    TrimEdge { leftEdge: true; visible: item.bodyWidth > 20 }
    TrimEdge { leftEdge: false; visible: item.bodyWidth > 20 }

    // ------------------------------------------------------------- loop
    // The top-right corner: drag right to repeat the region, double-click to stop.
    Item {
        id: loopHandle
        x: item.width - width
        width: 18; height: 18
        z: 6
        visible: (item.selected || loopArea.containsMouse || moveArea.containsMouse || item.looped) && item.bodyWidth > 36
        Rectangle {
            anchors.fill: parent
            anchors.margins: 2
            radius: 4
            color: loopArea.containsMouse || loopArea.pressed ? "#FFFFFF" : Qt.rgba(0, 0, 0, 0.25)
            Icon {
                anchors.centerIn: parent
                glyph: Icons.repeat
                size: 10
                color: loopArea.containsMouse || loopArea.pressed ? Qt.darker(item.tint, 1.8) : "#FFFFFF"
            }
        }
        MouseArea {
            id: loopArea
            anchors.fill: parent
            anchors.margins: -3
            hoverEnabled: true
            preventStealing: true
            cursorShape: Qt.SizeHorCursor
            onPressed: if (!item.selected) item.app.selectRegion(item.region.id)
            onPositionChanged: m => {
                if (!pressed) return
                item.busy = true
                const end = item.app.snapBeats(item.region.start + mapToItem(item, m.x, 0).x / item.pxPerBeat)
                item.previewSpan = Math.max(item.region.beats, end - item.region.start)
            }
            onReleased: {
                const app = item.app, id = item.region.id, busy = item.busy, end = item.region.start + item.previewSpan
                item.reset()
                if (busy) app.setRegionLoop(id, end)
            }
            onDoubleClicked: item.app.setRegionLoop(item.region.id, item.region.start)
        }
        DToolTip {
            visible: loopArea.containsMouse && !loopArea.pressed
            text: item.looped ? "Loop · drag to change, double-click to stop looping" : "Loop · drag right to repeat this region"
        }
    }

    // ------------------------------------------------------------- fades
    // A round handle where each fade curve meets the top of the region.
    // Drag it to set the fade; double-click to remove the fade.
    component FadeHandle: Item {
        id: handle
        property bool fadeIn: true
        readonly property real fadeBeats: fadeIn ? item.previewFadeIn : item.previewFadeOut
        readonly property real secs: fadeBeats * item.region.length / Math.max(1e-9, item.region.beats)
        readonly property bool hot: hover.containsMouse || hover.pressed
        readonly property bool dragging: hover.pressed && item.busy

        width: 16; height: 16
        y: 18 - height / 2
        x: (fadeIn ? fadeBeats * item.pxPerBeat : item.bodyWidth - fadeBeats * item.pxPerBeat) - width / 2
        visible: !item.isMidi && (item.selected || hot || moveArea.containsMouse) && item.bodyWidth > 30
        z: 5

        Rectangle {
            anchors.centerIn: parent
            width: handle.hot ? 16 : 13
            height: width
            radius: width / 2
            color: "#FFFFFF"
            border.color: Qt.darker(item.tint, 1.7)
            border.width: 1.5
            Behavior on width { NumberAnimation { duration: Theme.fast } }
            // A tiny ramp: up for a fade-in, down for a fade-out.
            Shape {
                anchors.centerIn: parent
                width: 7; height: 6
                preferredRendererType: Shape.CurveRenderer
                ShapePath {
                    strokeColor: Qt.darker(item.tint, 1.7)
                    strokeWidth: 1.4
                    fillColor: "transparent"
                    capStyle: ShapePath.RoundCap
                    startX: 0; startY: handle.fadeIn ? 6 : 0
                    PathQuad { x: 7; y: handle.fadeIn ? 0 : 6; controlX: handle.fadeIn ? 1 : 6; controlY: handle.fadeIn ? 0 : 0 }
                }
            }
        }

        MouseArea {
            id: hover
            anchors.fill: parent
            anchors.margins: -9
            hoverEnabled: true
            preventStealing: true
            cursorShape: Qt.SizeHorCursor
            // Where the pointer grabbed the handle relative to its centre, so
            // the fade doesn't jump to the pointer on the first move.
            property real grab: 0
            onPressed: m => {
                if (!item.selected) item.app.selectRegion(item.region.id)
                grab = mapToItem(item, m.x, 0).x - (handle.x + handle.width / 2)
            }
            onPositionChanged: m => {
                if (!pressed) return
                item.busy = true
                // 1:1 with the pointer, measured in the region's own coordinates.
                const x = mapToItem(item, m.x, 0).x - grab
                const len = item.region.beats
                if (handle.fadeIn) item.previewFadeIn = Math.max(0, Math.min(len - item.previewFadeOut, x / item.pxPerBeat))
                else item.previewFadeOut = Math.max(0, Math.min(len - item.previewFadeIn, (item.bodyWidth - x) / item.pxPerBeat))
            }
            onReleased: {
                const app = item.app, id = item.region.id, busy = item.busy
                const fin = item.previewFadeIn, fout = item.previewFadeOut
                item.reset()
                if (busy) app.setRegionFades(id, fin, fout, true)
            }
            onDoubleClicked: item.app.setRegionFades(item.region.id, handle.fadeIn ? 0 : item.region.fadeIn,
                                                     handle.fadeIn ? item.region.fadeOut : 0, true)
        }

        DToolTip {
            visible: handle.hot
            text: (handle.fadeIn ? "Fade in" : "Fade out") + " · "
                  + (handle.secs < 1 ? Math.round(handle.secs * 1000) + " ms" : handle.secs.toFixed(2) + " s")
                  + (hover.pressed ? "" : " · drag to change, double-click to remove")
            delay: 0
        }
    }
    FadeHandle { fadeIn: true }
    FadeHandle { fadeIn: false }

    // Follow the song's tempo: what tempo the audio is at (detected, or typed).
    Popup {
        id: tempoPopup
        x: Math.min(item.width - width, 10)
        y: 22
        padding: 12
        onAboutToShow: {
            const found = item.app.detectRegionTempo(item.region.id)
            bpmField.text = (found > 0 ? found : item.app.tempo).toFixed(1)
            detected.text = found > 0 ? "Sounds like " + found.toFixed(1) + " BPM" : "No clear beat found: type its tempo"
        }
        background: GlassPanel { radius: Theme.radiusLg; clarity: 0.3 }
        contentItem: Column {
            spacing: 8
            width: 240
            DText { text: "Follow the song's tempo"; strong: true; font.pixelSize: Theme.textSm }
            DText {
                width: parent.width
                wrapMode: Text.WordWrap
                text: "This audio stays in time when you change the tempo, without changing its pitch."
                font.pixelSize: Theme.textXs
                dim: true
            }
            Row {
                spacing: 8
                DText { anchors.verticalCenter: parent.verticalCenter; text: "It's at"; font.pixelSize: Theme.textSm }
                Rectangle {
                    width: 70; height: 26; radius: Theme.radiusSm
                    color: Theme.tint(Theme.sunken, 0.8)
                    border.color: bpmField.activeFocus ? Theme.accent : Theme.line
                    TextInput {
                        id: bpmField
                        anchors.fill: parent
                        anchors.margins: 6
                        verticalAlignment: TextInput.AlignVCenter
                        font.family: Theme.mono
                        font.pixelSize: Theme.textSm
                        color: Theme.text
                        validator: DoubleValidator { bottom: 20; top: 400; decimals: 1 }
                        onAccepted: applyTempo.clicked()
                    }
                }
                DText { anchors.verticalCenter: parent.verticalCenter; text: "BPM"; font.pixelSize: Theme.textSm; dim: true }
            }
            DText { id: detected; font.pixelSize: Theme.textXs; faint: true }
            Row {
                spacing: 6
                DButton {
                    variant: "ghost"; implicitHeight: 26; text: "Half"
                    onClicked: bpmField.text = (parseFloat(bpmField.text) / 2).toFixed(1)
                }
                DButton {
                    variant: "ghost"; implicitHeight: 26; text: "Double"
                    onClicked: bpmField.text = (parseFloat(bpmField.text) * 2).toFixed(1)
                }
                DButton {
                    id: applyTempo
                    variant: "primary"; implicitHeight: 26; text: "Follow"
                    onClicked: {
                        const bpm = parseFloat(bpmField.text)
                        tempoPopup.close()
                        if (bpm >= 20 && bpm <= 400) item.app.setRegionTempo(item.region.id, bpm)
                    }
                }
            }
        }
    }

    // Region gain: a slider from −24 to +24 dB.
    Popup {
        id: gainPopup
        x: Math.min(item.width - width, 10)
        y: 22
        padding: 12
        background: GlassPanel { radius: Theme.radiusLg; clarity: 0.3 }
        contentItem: Column {
            spacing: 8
            Row {
                spacing: 8
                DText { text: "Region gain"; strong: true; font.pixelSize: Theme.textSm }
                DText {
                    text: (gainSlider.value > 0.05 ? "+" : "") + gainSlider.value.toFixed(1) + " dB"
                    font.family: Theme.mono
                    font.pixelSize: Theme.textSm
                    dim: true
                }
            }
            DSlider {
                id: gainSlider
                width: 200
                implicitHeight: 18
                from: -24; to: 24
                value: item.region.gain || 0
                onMoved: item.app.setRegionGain(item.region.id, value, false)
                onPressedChanged: if (!pressed) item.app.setRegionGain(item.region.id, value, true)
            }
            Row {
                spacing: 6
                DButton { variant: "ghost"; implicitHeight: 24; text: "0 dB"; onClicked: item.app.setRegionGain(item.region.id, 0, true) }
                DButton { variant: "ghost"; implicitHeight: 24; text: "Normalize"; onClicked: { item.app.selectRegion(item.region.id); item.app.normalizeSelected() } }
            }
        }
    }

    MenuPopup {
        id: regionMenu
        width: 220
        items: {
            const mod = Qt.platform.os === "osx" ? "⌘" : "Ctrl+"
            const many = item.app.selectionCount > 1
            return [
                { label: "Cut", shortcut: mod + "X", action: () => item.app.cutSelected() },
                { label: "Copy", shortcut: mod + "C", action: () => item.app.copySelected() },
                { label: "Paste", shortcut: mod + "V", enabled: item.app.canPaste, action: () => item.app.paste() },
                { label: "Duplicate", shortcut: mod + "D", action: () => item.app.duplicateSelected() },
                { separator: true },
                { label: "Split at Playhead", shortcut: mod + "T", action: () => item.app.splitAtPlayhead() },
                { label: "Join", shortcut: mod + "J", enabled: many, action: () => item.app.joinSelected() },
                { label: many ? "Cycle These Regions" : "Cycle This Region", shortcut: mod + "U", action: () => item.app.cycleSelection() },
                { label: item.looped ? "Stop Looping" : "Loop 4 Times",
                  action: () => item.app.setRegionLoop(item.region.id, item.region.start + (item.looped ? 0 : item.region.beats * 4)) },
                { separator: true },
                { label: "Gain…", enabled: !item.isMidi, action: () => gainPopup.open() },
                { label: "Normalize", enabled: !item.isMidi, action: () => item.app.normalizeSelected() },
                { label: item.region.reverse ? "Play Forwards" : "Reverse", enabled: !item.isMidi, action: () => item.app.reverseSelected() },
                { label: item.region.tempo ? "Stop Following Tempo" : "Follow Song Tempo…", enabled: !item.isMidi,
                  action: () => item.region.tempo ? item.app.setRegionTempo(item.region.id, 0) : tempoPopup.open() },
                { label: "Split into Stems…", enabled: !item.isMidi && !many && item.app.stemRegion < 0, action: () => item.stemsRequested() },
                { label: "Remove Fades", enabled: !many && (item.region.fadeIn > 0 || item.region.fadeOut > 0),
                  action: () => item.app.setRegionFades(item.region.id, 0, 0, true) },
                { separator: true },
                { label: many ? "Delete " + item.app.selectionCount + " Regions" : "Delete Region",
                  shortcut: "⌫", danger: true, action: () => item.app.deleteSelected() }
            ]
        }
    }
}
