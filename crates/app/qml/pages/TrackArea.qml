import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Isai

// The arrangement: a toolbar, the bar ruler, track headers and lanes.
//
// Zoom: ⌘ + scroll (around the mouse), ⌘= / ⌘−, the toolbar buttons, or Z to
// fit the whole song. Scroll in both directions; the view follows the
// playhead while playing. Only what's on screen is drawn.
Rectangle {
    id: area
    property var app
    signal importRequested()
    signal mixerToggled()
    signal pianoRollToggled()
    signal playlistRequested()
    signal tunerToggled()
    signal browserToggled()
    property bool browserShown: false
    signal shortcutsRequested()
    property bool tunerShown: false
    /// The tempo and signature lanes under the markers (G).
    property bool globalShown: false
    signal globalToggled()
    property bool pianoRollShown: false
    /// A MIDI region was double-clicked (or created): open the piano roll.
    signal editRequested(int regionId)
    signal stemsRequested(int regionId, string name)
    /// Open a track's instrument window.
    signal instrumentRequested(int trackId)
    /// What the "+" menus offer, grouped: one list for both.
    readonly property var newTrackItems: [
        { header: "AUDIO" },
        { label: "Audio track", shortcut: "⌥⌘N", action: () => app.addTrack() },
        { separator: true },
        { header: "INSTRUMENTS" },
        { label: "Piano", shortcut: "Isai Piano", action: () => addInstrument("piano") },
        { label: "Synth", shortcut: "Isai Synth", action: () => addInstrument("synth") },
        { label: "Drum machine", shortcut: "Isai Drums", action: () => addInstrument("drums") },
        { label: "Sampler", shortcut: "Isai Sampler", action: () => addInstrument("sampler") }
    ].concat(pluginTrackItems)
    /// Instrument plugins installed on this computer (VST3), then a rescan.
    readonly property var pluginList: {
        try { return JSON.parse(app.pluginsJson || "[]") } catch (e) { return [] }
    }
    readonly property var pluginTrackItems: {
        const out = [{ separator: true }, { header: "PLUGIN INSTRUMENTS" }]
        for (const p of pluginList.filter(x => x.instrument))
            out.push({ label: p.name, shortcut: p.vendor, action: () => app.addPluginTrack(p.index) })
        out.push({ label: app.pluginScanning ? "Looking for plugins…" : "Rescan plugins", enabled: !app.pluginScanning, action: () => app.rescanPlugins() })
        return out
    }
    function addInstrument(kind) {
        app.addInstrumentTrack(kind)
        instrumentRequested(app.selectedTrack)
    }
    property bool mixerShown: false
    /// Single-key shortcuts (off while the computer keyboard plays notes).
    property bool keysEnabled: true
    /// Space covered by the docked mixer at the bottom.
    property real bottomInset: 0
    color: Theme.bg

    readonly property int headerWidth: 236
    readonly property int laneHeight: 90
    readonly property int pad: 12
    readonly property int columnWidth: app.waveformDetail === "high" ? 1 : app.waveformDetail === "low" ? 4 : 2

    readonly property real timelineX: headerWidth + pad * 2
    readonly property real timelineWidth: Math.max(100, width - timelineX - pad)

    // Zoom: 1 fits the whole timeline; larger zooms in.
    property real zoom: 1
    readonly property real fitPx: timelineWidth / Math.max(1, app.lengthBeats)
    readonly property real maxPx: 160
    readonly property real pxPerBeat: fitPx * zoom

    readonly property var tracks: {
        try { return JSON.parse(app.tracksJson || "[]") } catch (e) { return [] }
    }
    /// Each track's [left, right] level, in track order.
    readonly property var meters: {
        try { return JSON.parse(app.trackMeters || "[]") } catch (e) { return [] }
    }
    /// Selected region ids as a lookup: selectedIds[id] === true.
    readonly property var selectedIds: {
        const ids = {}
        try { for (const id of JSON.parse(app.selectedRegions || "[]")) ids[id] = true } catch (e) {}
        return ids
    }
    /// How far the selection can move: its earliest start and its top and bottom track rows.
    readonly property var selectionBounds: {
        let minStart = Infinity, top = Infinity, bottom = -1
        tracks.forEach((t, row) => t.regions.forEach(r => {
            if (!selectedIds[r.id]) return
            minStart = Math.min(minStart, r.start)
            top = Math.min(top, row)
            bottom = Math.max(bottom, row)
        }))
        return { minStart: minStart, top: top, bottom: bottom }
    }

    // While one selected region is dragged, the rest of the selection follows
    // by the same amount (in beats and track rows).
    property bool groupDragging: false
    property real groupDx: 0
    property int groupDy: 0

    /// Height of an open automation lane under a track.
    readonly property int autoHeight: 78
    function rowHeight(i) { return laneHeight + (tracks[i] && tracks[i].automationView ? autoHeight : 0) }
    /// Where each track's row starts (one extra entry: the end of the last row).
    readonly property var rowTops: {
        const tops = [0]
        for (let i = 0; i < tracks.length; ++i) tops.push(tops[i] + rowHeight(i))
        return tops
    }
    readonly property real rowsHeight: rowTops[tracks.length]
    function rowAt(y) {
        let i = 0
        while (i < tracks.length - 1 && rowTops[i + 1] <= y) ++i
        return Math.max(0, i)
    }

    function zoomBy(factor, anchorX) {
        // Keep the beat under `anchorX` (relative to the visible timeline) in place.
        const ax = anchorX === undefined ? lanes.width / 2 : anchorX
        const beat = (lanes.contentX + ax) / pxPerBeat
        zoom = Math.max(1, Math.min(maxPx / fitPx, zoom * factor))
        lanes.contentX = Math.max(0, Math.min(lanes.contentWidth - lanes.width, beat * pxPerBeat - ax))
    }
    function zoomToFit() { zoom = 1; lanes.contentX = 0 }
    function openHistory() { historyPopup.open() }
    /// Files dropped at (x, y) in this view: onto the track lane there, at that
    /// beat (into a sampler's slot for a sampler track). False when not over a lane.
    function dropFiles(urls, x, y) {
        if (x < timelineX || y < lanesY || urls.length === 0) return false
        const beat = app.snapBeats((x - timelineX + lanes.contentX) / pxPerBeat)
        const ly = y - lanesY + lanes.contentY
        const t = ly < rowsHeight ? tracks[rowAt(ly)] : null
        urls.forEach((u, i) => {
            const url = String(u)
            if (i === 0 && t && t.instrument && t.instrument.kind === "sampler") app.loadSample(t.id, url)
            else app.importAt(url, i === 0 && t && !t.instrument ? t.id : -1, beat)
        })
        return true
    }

    // ------------------------------------------------------------- toolbar
    Rectangle {
        id: toolbar
        width: parent.width
        height: 38
        color: Theme.panel
        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.line }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: area.pad + 4
            anchors.rightMargin: area.pad
            spacing: 6
            DText {
                text: "TRACKS"
                faint: true
                font.pixelSize: Theme.textXs
                font.weight: Font.DemiBold
                font.letterSpacing: 1.2
            }
            DText {
                text: area.app.trackCount
                faint: true
                font.family: Theme.mono
                font.pixelSize: Theme.textXs
            }
            IconButton {
                implicitWidth: 28; implicitHeight: 26
                iconSize: 15
                glyph: Icons.plus
                tip: "New track (⌥⌘N)"
                onClicked: newTrackMenu.open()
                MenuPopup {
                    id: newTrackMenu
                    y: parent.height + 4
                    width: 230
                    items: area.newTrackItems
                }
            }
            Item { Layout.fillWidth: true }

            // Windows, FL Studio style: Playlist (F5), Piano Roll (F7), Mixer (F9).
            Rectangle {
                implicitWidth: windowRow.implicitWidth + 6
                implicitHeight: 30
                radius: Theme.radiusMd + 2
                color: Theme.tint(Theme.sunken, 0.8)
                border.color: Theme.line
                Row {
                    id: windowRow
                    anchors.centerIn: parent
                    spacing: 2
                    Repeater {
                        model: [
                            { glyph: Icons.waveform, label: "Playlist", key: "F5", on: !area.pianoRollShown && !area.mixerShown,
                              tip: "Just the tracks", act: () => area.playlistRequested() },
                            { glyph: Icons.pianoKeys, label: "Piano Roll", key: "F7", on: area.pianoRollShown,
                              tip: "Opens the selected MIDI region, or makes one at the playhead", act: () => area.pianoRollToggled() },
                            { glyph: Icons.sliders, label: "Mixer", key: "F9", on: area.mixerShown,
                              tip: "Faders, sends and effects", act: () => area.mixerToggled() }
                        ]
                        delegate: AbstractButton {
                            id: win
                            required property var modelData
                            implicitHeight: 24
                            implicitWidth: winContent.implicitWidth + 18
                            hoverEnabled: true
                            onClicked: modelData.act()
                            background: Rectangle {
                                radius: Theme.radiusMd
                                color: win.modelData.on ? Theme.accentSoft : win.hovered ? Theme.hover : "transparent"
                                border.color: win.modelData.on ? Theme.tint(Theme.accent, 0.5) : "transparent"
                            }
                            contentItem: Row {
                                id: winContent
                                spacing: 6
                                leftPadding: 9
                                Icon {
                                    anchors.verticalCenter: parent.verticalCenter
                                    glyph: win.modelData.glyph
                                    size: 14
                                    color: win.modelData.on ? Theme.accent : Theme.textDim
                                }
                                DText {
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: win.modelData.label
                                    font.pixelSize: Theme.textSm
                                    color: win.modelData.on ? Theme.text : Theme.textDim
                                }
                                DText {
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: win.modelData.key
                                    font.family: Theme.mono
                                    font.pixelSize: 10
                                    faint: true
                                }
                            }
                            DToolTip { visible: win.hovered; text: win.modelData.label + " (" + win.modelData.key + "): " + win.modelData.tip }
                        }
                    }
                }
            }

            Item { Layout.fillWidth: true }

            IconButton {
                implicitWidth: 28; implicitHeight: 26; iconSize: 15
                glyph: Icons.folderOpen
                active: area.browserShown
                tip: "Browser: find and hear sounds on your computer (B)"
                onClicked: area.browserToggled()
            }
            IconButton {
                id: historyButton
                implicitWidth: 28; implicitHeight: 26; iconSize: 15
                glyph: Icons.clock
                active: historyPopup.opened
                tip: "Undo history: jump back several steps (⌥⌘Z)"
                onClicked: historyPopup.opened ? historyPopup.close() : historyPopup.open()
                HistoryPopup { id: historyPopup; app: area.app; y: parent.height + 4; x: parent.width - width }
            }
            IconButton {
                implicitWidth: 28; implicitHeight: 26; iconSize: 15
                glyph: Icons.metronome
                active: area.globalShown
                tip: "Tempo and time-signature changes over the song (G)"
                onClicked: area.globalToggled()
            }
            IconButton {
                implicitWidth: 28; implicitHeight: 26; iconSize: 15
                glyph: Icons.gauge
                active: area.tunerShown
                tip: "Tuner: tune a guitar, bass or voice from your input"
                onClicked: area.tunerToggled()
            }
            IconButton {
                implicitWidth: 28; implicitHeight: 26; iconSize: 15
                glyph: Icons.keyboard
                tip: "Keyboard shortcuts (?)"
                onClicked: area.shortcutsRequested()
            }
            Rectangle { Layout.preferredWidth: 1; Layout.preferredHeight: 20; color: Theme.line }
            IconButton {
                implicitWidth: 28; implicitHeight: 26; iconSize: 15
                glyph: Icons.scissors
                tip: "Split at playhead (⌘T) · ⌥-click a region to split there"
                onClicked: area.app.splitAtPlayhead()
            }
            Rectangle { Layout.preferredWidth: 1; Layout.preferredHeight: 20; color: Theme.line }
            IconButton {
                implicitWidth: 28; implicitHeight: 26; iconSize: 15
                glyph: Icons.zoomOut
                tip: "Zoom out (⌘−)"
                enabled: area.zoom > 1
                onClicked: area.zoomBy(1 / 1.5)
            }
            IconButton {
                implicitWidth: 28; implicitHeight: 26; iconSize: 15
                glyph: Icons.zoomFit
                tip: "Fit the whole song (Z)"
                onClicked: area.zoomToFit()
            }
            IconButton {
                implicitWidth: 28; implicitHeight: 26; iconSize: 15
                glyph: Icons.zoomIn
                tip: "Zoom in (⌘=) · ⌘ + scroll to zoom at the mouse"
                enabled: area.pxPerBeat < area.maxPx
                onClicked: area.zoomBy(1.5)
            }
            Rectangle { Layout.preferredWidth: 1; Layout.preferredHeight: 20; color: Theme.line }
            IconButton {
                implicitWidth: 30; implicitHeight: 28
                iconSize: 16
                glyph: Icons.magnet
                active: area.app.snap
                tip: area.app.snap ? "Snap to grid is on (N)" : "Snap to grid (N)"
                onClicked: area.app.toggleSnap()
            }
            Segmented {
                implicitHeight: 28
                minItemWidth: 44
                opacity: area.app.snap ? 1 : 0.5
                options: [
                    { value: "bar", label: "Bar" },
                    { value: "beat", label: "Beat" },
                    { value: "1/2", label: "½" },
                    { value: "1/4", label: "¼" }
                ]
                value: area.app.snapValue
                onActivated: v => area.app.setSnapGrid(v)
            }
        }
    }

    // ------------------------------------------------------------- ruler
    Item {
        id: rulerView
        x: area.timelineX
        y: toolbar.height + 6
        width: area.timelineWidth
        height: ruler.height
        clip: true
        Ruler {
            id: ruler
            x: -lanes.contentX
            width: lanes.contentWidth
            app: area.app
            pxPerBeat: area.pxPerBeat
            viewStart: lanes.contentX
            viewWidth: rulerView.width
            globalShown: area.globalShown
        }
    }

    readonly property real lanesY: rulerView.y + rulerView.height + 6

    // ------------------------------------------------------------- headers
    // A separate column that follows the lanes vertically but never scrolls sideways.
    Flickable {
        id: headers
        x: area.pad
        y: area.lanesY
        width: area.headerWidth
        height: area.height - y
        contentHeight: lanes.contentHeight
        contentY: lanes.contentY
        interactive: false
        clip: true
        // Scrolling over the headers scrolls the tracks.
        WheelHandler {
            onWheel: e => lanes.contentY = Math.max(0, Math.min(lanes.contentHeight - lanes.height,
                                                                 lanes.contentY - e.angleDelta.y / 2))
        }

        Repeater {
            model: area.tracks
            delegate: Item {
                id: headerRow
                required property var modelData
                required property int index
                y: area.rowTops[index] + 3
                z: header.dragging ? 10 : 0
                width: area.headerWidth
                height: area.laneHeight - 6
                TrackHeader {
                    id: header
                    anchors.fill: parent
                    app: area.app
                    track: headerRow.modelData
                    index: headerRow.index
                    count: area.tracks.length
                    rowHeight: area.laneHeight
                    level: area.meters[headerRow.index] || [0, 0]
                    selected: area.app.selectedTrack === headerRow.modelData.id
                    onInstrumentRequested: area.instrumentRequested(headerRow.modelData.id)
                }
                // The open automation lane's picker.
                Item {
                    visible: !!headerRow.modelData.automationView
                    y: area.laneHeight - 2
                    width: parent.width
                    height: area.autoHeight - 6
                    readonly property var targets: headerRow.modelData.autoTargets || []
                    readonly property int current: Math.max(0, targets.findIndex(t => t.key === headerRow.modelData.automationView))
                    Rectangle {
                        anchors.fill: parent
                        radius: Theme.radiusMd
                        color: Theme.tint(Theme.trackColor(headerRow.modelData.color), 0.06)
                        border.color: Theme.tint(Theme.line, 0.8)
                    }
                    Column {
                        anchors.fill: parent
                        anchors.margins: 8
                        anchors.leftMargin: 20
                        spacing: 6
                        Dropdown {
                            width: parent.width
                            implicitHeight: 28
                            font.pixelSize: Theme.textSm
                            model: parent.parent.targets.map(t => t.label + (t.points.length > 0 ? "  •" : ""))
                            currentIndex: parent.parent.current
                            onActivated: i => area.app.setAutomationView(headerRow.modelData.id, parent.parent.targets[i].key)
                        }
                        Row {
                            spacing: 6
                            DText {
                                anchors.verticalCenter: parent.verticalCenter
                                readonly property var t: parent.parent.parent.targets[parent.parent.parent.current]
                                text: t ? Auto.format(t, Auto.live(headerRow.modelData, t.key, area.app.positionBeats, true, t.value)) : ""
                                font.family: Theme.mono
                                font.pixelSize: Theme.textXs
                                dim: true
                            }
                            DButton {
                                variant: "ghost"
                                implicitHeight: 22
                                text: "Clear"
                                visible: { const t = parent.parent.parent.targets[parent.parent.parent.current]; return !!t && t.points.length > 0 }
                                onClicked: area.app.clearLane(headerRow.modelData.id, headerRow.modelData.automationView)
                            }
                        }
                    }
                }
            }
        }

        AbstractButton {
            visible: area.tracks.length > 0
            y: area.rowsHeight + 8
            width: area.headerWidth
            height: 36
            hoverEnabled: true
            onClicked: bottomTrackMenu.open()
            MenuPopup {
                id: bottomTrackMenu
                y: parent.height + 4
                width: 230
                items: area.newTrackItems
            }
            background: Rectangle {
                radius: Theme.radiusMd
                color: parent.hovered ? Theme.raised : "transparent"
                border.color: Theme.line
            }
            contentItem: Row {
                spacing: 8
                leftPadding: 12
                Icon { glyph: Icons.plus; size: 14; color: Theme.textDim; anchors.verticalCenter: parent.verticalCenter }
                DText { text: "New track"; dim: true; anchors.verticalCenter: parent.verticalCenter }
            }
        }
    }

    // ------------------------------------------------------------- lanes
    Flickable {
        id: lanes
        x: area.timelineX
        y: area.lanesY
        width: area.timelineWidth
        height: area.height - y
        contentWidth: Math.max(width, area.app.lengthBeats * area.pxPerBeat)
        contentHeight: Math.max(height, area.rowsHeight + 60 + area.bottomInset)
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        flickableDirection: Flickable.HorizontalAndVerticalFlick
        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
        ScrollBar.horizontal: ScrollBar { policy: area.zoom > 1.001 ? ScrollBar.AlwaysOn : ScrollBar.AsNeeded }

        // ⌘ + scroll zooms around the mouse.
        WheelHandler {
            acceptedModifiers: Qt.ControlModifier
            // Mouse wheels report angleDelta; Mac trackpads report smooth pixelDelta.
            onWheel: e => {
                const d = e.angleDelta.y !== 0 ? e.angleDelta.y / 120 : e.pixelDelta.y / 40
                if (d !== 0) area.zoomBy(Math.pow(1.15, d), e.x - lanes.contentX)
            }
        }

        // Grid: bar lines, plus beat lines when there is room (visible range only).
        Item {
            id: grid
            width: lanes.contentWidth
            height: lanes.contentHeight
            // Bars in view (they change length with the time signature); beat
            // lines inside them when there's room, every Nth bar when zoomed far out.
            readonly property var bars: TimeMap.barsBetween(lanes.contentX / area.pxPerBeat, (lanes.contentX + lanes.width) / area.pxPerBeat, 800)
            readonly property real barPx: TimeMap.barLenAt(lanes.contentX / area.pxPerBeat) * area.pxPerBeat
            readonly property int every: barPx >= 6 ? 1 : Math.pow(2, Math.ceil(Math.log2(6 / Math.max(0.1, barPx))))
            Repeater {
                model: grid.bars
                delegate: Item {
                    id: barLine
                    required property var modelData
                    readonly property real beatPx: modelData.unit * area.pxPerBeat
                    x: modelData.beat * area.pxPerBeat
                    height: grid.height
                    Rectangle {
                        visible: (barLine.modelData.bar - 1) % grid.every === 0
                        width: 1
                        height: parent.height
                        color: Theme.tint(Theme.lineStrong, 0.55)
                    }
                    Repeater {
                        model: barLine.beatPx >= 10 ? barLine.modelData.num - 1 : 0
                        delegate: Rectangle {
                            required property int index
                            x: (index + 1) * barLine.beatPx
                            width: 1
                            height: barLine.height
                            color: Theme.tint(Theme.line, 0.6)
                        }
                    }
                }
            }
            // Marker lines down through the tracks.
            Repeater {
                model: ruler.markers
                delegate: Rectangle {
                    required property var modelData
                    x: modelData.beat * area.pxPerBeat
                    width: 1
                    height: grid.height
                    color: Theme.tint(Theme.marker, 0.45)
                }
            }
            Rectangle {
                visible: area.app.cycle
                x: area.app.cycleStart * area.pxPerBeat
                width: (area.app.cycleEnd - area.app.cycleStart) * area.pxPerBeat
                height: grid.height
                color: Theme.tint(Theme.loopOn, 0.06)
            }
        }

        // Empty space: click to move the playhead and select that track; drag a
        // box to select every region it touches (⇧ adds to the selection).
        MouseArea {
            id: marquee
            width: lanes.contentWidth
            height: lanes.contentHeight
            preventStealing: true
            property point origin
            property point now
            property bool dragging: false

            function select(add) {
                if (area.tracks.length === 0) return
                area.app.selectRegionsIn(origin.x / area.pxPerBeat, now.x / area.pxPerBeat,
                                         area.rowAt(origin.y), area.rowAt(now.y), add)
            }
            onPressed: m => { origin = Qt.point(m.x, m.y); now = origin; dragging = false }
            // Double-click an instrument track's empty lane: a new one-bar MIDI region there.
            onDoubleClicked: m => {
                const row = area.rowAt(m.y)
                const t = area.tracks[row]
                if (!t || !t.instrument || m.y >= area.rowTops[row] + area.laneHeight) return
                // A one-bar region, from the start of the bar under the mouse.
                const beat = m.x / area.pxPerBeat
                const id = area.app.addMidiRegion(t.id, TimeMap.barStart(TimeMap.barAt(beat)), TimeMap.barLenAt(beat))
                if (id >= 0) area.editRequested(id)
            }
            onPositionChanged: m => {
                if (!pressed) return
                now = Qt.point(m.x, m.y)
                if (!dragging && Math.abs(now.x - origin.x) < 4 && Math.abs(now.y - origin.y) < 4) return
                dragging = true
                if (!(m.modifiers & Qt.ShiftModifier)) select(false)
            }
            onReleased: m => {
                if (dragging) {
                    select(!!(m.modifiers & Qt.ShiftModifier))
                } else {
                    const row = m.y < area.rowsHeight ? area.rowAt(m.y) : area.tracks.length
                    if (row < area.tracks.length) area.app.selectTrack(area.tracks[row].id)
                    else area.app.selectRegion(-1)
                    area.app.seekBeats(area.app.snapBeats(m.x / area.pxPerBeat))
                }
                dragging = false
            }
        }

        Repeater {
            model: area.tracks
            delegate: Rectangle {
                id: lane
                required property var modelData
                required property int index
                // Vivid: regions fill the lane, so neighbouring tracks touch.
                readonly property int gap: Theme.vivid ? 1 : 3
                y: area.rowTops[index] + gap
                width: lanes.contentWidth
                height: area.laneHeight - gap * 2
                radius: Theme.radiusSm
                color: area.app.selectedTrack === modelData.id
                       ? Theme.tint(Theme.trackColor(modelData.color), 0.07) : "transparent"
                border.color: Theme.vivid ? "transparent" : Theme.tint(Theme.line, 0.6)
                // The lane holding the dragged region draws above the others.
                z: area.app.selectedTrack === modelData.id ? 1 : 0
                AutomationLane {
                    visible: !!lane.modelData.automationView
                    y: area.laneHeight - lane.gap + 2
                    width: parent.width
                    height: area.autoHeight - 6
                    app: area.app
                    track: lane.modelData
                    target: Auto.target(lane.modelData, lane.modelData.automationView)
                    pxPerBeat: area.pxPerBeat
                    tint: Theme.trackColor(lane.modelData.color)
                    viewStart: lanes.contentX
                    viewWidth: lanes.width
                }
                Repeater {
                    model: lane.modelData.regions
                    delegate: RegionItem {
                        required property var modelData
                        region: modelData
                        app: area.app
                        tint: Theme.trackColor(lane.modelData.color)
                        frozen: !!lane.modelData.frozen
                        pxPerBeat: area.pxPerBeat
                        laneHeight: area.laneHeight
                        rowTops: area.rowTops
                        trackIndex: lane.index
                        tracks: area.tracks
                        view: area
                        onEditRequested: area.editRequested(modelData.id)
                        onStemsRequested: area.stemsRequested(modelData.id, modelData.name)
                        selected: area.selectedIds[modelData.id] === true
                        columnWidth: area.columnWidth
                        // Only regions near the screen are drawn.
                        visible: x + width >= lanes.contentX - 50 && x <= lanes.contentX + lanes.width + 50
                        height: parent.height
                    }
                }
            }
        }

        // The marquee box
        Rectangle {
            visible: marquee.dragging
            x: Math.min(marquee.origin.x, marquee.now.x)
            y: Math.min(marquee.origin.y, marquee.now.y)
            width: Math.abs(marquee.now.x - marquee.origin.x)
            height: Math.abs(marquee.now.y - marquee.origin.y)
            z: 20
            color: Theme.tint(Theme.accent, 0.12)
            border.color: Theme.accent
            radius: 2
        }
    }

    // Empty arrangement hint
    Column {
        visible: area.tracks.length === 0 && !area.app.loading
        anchors.centerIn: parent
        anchors.verticalCenterOffset: 30
        spacing: 12
        Icon {
            anchors.horizontalCenter: parent.horizontalCenter
            glyph: Icons.waveform
            size: 34
            color: Theme.textFaint
        }
        DText {
            anchors.horizontalCenter: parent.horizontalCenter
            text: "This project is empty"
            strong: true
            font.pixelSize: Theme.textLg
        }
        DText {
            anchors.horizontalCenter: parent.horizontalCenter
            text: "Import audio or add a track to begin. Press Play to hear the metronome."
            dim: true
        }
        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: 8
            DButton { glyph: Icons.downloadSimple; text: "Import audio"; onClicked: area.importRequested() }
            DButton { glyph: Icons.plus; text: "Audio track"; onClicked: area.app.addTrack() }
            DButton { glyph: Icons.pianoKeys; text: "Piano track"; onClicked: area.addInstrument("piano") }
            DButton { glyph: Icons.metronome; text: "Drum track"; onClicked: area.addInstrument("drums") }
        }
    }

    // ------------------------------------------------------------- playhead
    Item {
        x: area.timelineX
        y: rulerView.y + ruler.cycleY + ruler.stripHeight
        width: area.timelineWidth
        height: area.height - y
        clip: true
        Item {
            x: area.app.positionBeats * area.pxPerBeat - lanes.contentX - 1
            width: 2
            height: parent.height
            Rectangle { anchors.fill: parent; color: Theme.playhead; opacity: 0.9 }
            Rectangle {
                width: 11; height: 11; radius: 2
                rotation: 45
                x: -4.5; y: -3
                color: Theme.playhead
            }
        }
    }

    // Follow the playhead while playing: turn the page when it reaches the edge.
    Connections {
        target: area.app
        function onPositionBeatsChanged() {
            if (!area.app.playing || area.zoom <= 1.001) return
            const x = area.app.positionBeats * area.pxPerBeat
            if (x > lanes.contentX + lanes.width * 0.92 || x < lanes.contentX)
                lanes.contentX = Math.max(0, Math.min(lanes.contentWidth - lanes.width, x - lanes.width * 0.08))
        }
    }

    Shortcut { sequences: ["Ctrl+=", "Ctrl++"]; enabled: area.visible; onActivated: area.zoomBy(1.5) }
    Shortcut { sequence: "Ctrl+-"; enabled: area.visible; onActivated: area.zoomBy(1 / 1.5) }
    Shortcut { sequence: "Z"; enabled: area.visible && area.keysEnabled; onActivated: area.zoomToFit() }
}
