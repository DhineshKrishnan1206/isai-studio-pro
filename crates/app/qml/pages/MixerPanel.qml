import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Isai

// The mixer: a glass pane docked along the bottom of the arrangement (X to
// show or hide), lined up with its edges. One channel strip per track, the
// master strip, and the selected track's effects. Drag the title bar up or
// down to resize it.
Item {
    id: mixer
    property var app
    property var tracks: []
    property bool shown: false
    /// What the frosted glass blurs (the arrangement).
    property Item backdrop: null
    signal closeRequested()
    /// An effect's editor window: its id, which chain it's on, and the chain's colour.
    signal effectEditRequested(int id, string chainName, color tint)
    signal masteringRequested()

    readonly property var buses: {
        try { return JSON.parse(app.busesJson || "[]") } catch (e) { return [] }
    }
    readonly property var meters: {
        try { return JSON.parse(app.trackMeters || "[]") } catch (e) { return [] }
    }
    readonly property var selectedTrack: tracks.find(t => t.id === app.selectedTrack) || null
    readonly property var masterInserts: {
        try { return JSON.parse(app.masterInsertsJson || "[]") } catch (e) { return [] }
    }

    // Which chain the effects column shows: the selected track (default),
    // a bus or the master. Clicking a strip picks it.
    property string chainType: "track"
    property int chainId: -1
    readonly property var focusedBus: chainType === "bus" ? buses.find(b => b.id === chainId) || null : null
    function openAddMenu() { effects.openAddMenu() }
    function focusTrack(id) { chainType = "track"; app.selectTrack(id) }
    function focusBus(id) { chainType = "bus"; chainId = id }
    function focusMaster() { chainType = "master"; chainId = -1 }
    /// Opens one effect in the effects column.
    /// Opens an effect's editor window.
    function openInsert(id) { effects.editRequested(id) }
    Connections {
        target: mixer.app
        function onSelectedTrackChanged() { if (mixer.app.selectedTrack >= 0) mixer.chainType = "track" }
    }
    // A deleted bus drops back to the track.
    onFocusedBusChanged: if (chainType === "bus" && !focusedBus) chainType = "track"

    readonly property int stripWidth: 70
    readonly property int effectsWidth: 244
    readonly property int masterWidth: 78
    readonly property real wanted: tracks.length * stripWidth + masterWidth + effectsWidth + 40

    /// Docked height; the arrangement leaves this much room below the last track.
    property real dockHeight: 380
    readonly property int margin: 12
    x: margin
    width: parent.width - margin * 2
    height: Math.max(240, Math.min(parent.height * 0.7, dockHeight))
    y: parent.height - height - margin

    visible: opacity > 0
    opacity: shown ? 1 : 0
    Behavior on opacity { NumberAnimation { duration: Theme.fast } }
    z: 50

    function dbText(db) { return db <= -60 ? "-∞" : (db > 0 ? "+" : "") + db.toFixed(1) }
    function toDb(gain) { return gain <= 0.001 ? -60 : Math.max(-60, 20 * Math.log10(gain)) }

    GlassPanel {
        id: panel
        anchors.fill: parent
        backdrop: mixer.backdrop
        // Swallow clicks so they don't reach the arrangement underneath.
        MouseArea { anchors.fill: parent; acceptedButtons: Qt.AllButtons; onWheel: w => w.accepted = true }

        // Title bar
        Item {
            id: titleBar
            width: parent.width
            height: 36
            // Drag up or down to resize.
            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.SizeVerCursor
                preventStealing: true
                property real startY
                property real startHeight
                onPressed: m => { startY = mapToItem(null, 0, m.y).y; startHeight = mixer.height }
                onPositionChanged: m => {
                    if (pressed) mixer.dockHeight = startHeight - (mapToItem(null, 0, m.y).y - startY)
                }
                onDoubleClicked: mixer.dockHeight = 380
            }
            // Grip
            Rectangle {
                anchors.horizontalCenter: parent.horizontalCenter
                y: 5
                width: 36; height: 4; radius: 2
                color: Theme.tint(Theme.text, 0.18)
            }
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 14
                anchors.rightMargin: 8
                spacing: 8
                Icon { glyph: Icons.sliders; size: 15; color: Theme.textDim }
                DText { text: "Mixer"; strong: true }
                LearnButton { topic: "mixer"; backdrop: mixer.backdrop; implicitWidth: 26; implicitHeight: 26; iconSize: 14 }
                DText { text: mixer.tracks.length + (mixer.tracks.length === 1 ? " track" : " tracks"); faint: true; font.pixelSize: Theme.textXs }
                Item { Layout.fillWidth: true }
                IconButton {
                    implicitWidth: 26; implicitHeight: 26; iconSize: 14
                    glyph: Icons.x
                    tip: "Hide the mixer (F9)"
                    onClicked: mixer.closeRequested()
                }
            }
            Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.line }
        }

        RowLayout {
            anchors.fill: parent
            anchors.topMargin: titleBar.height + 8
            anchors.bottomMargin: 10
            anchors.leftMargin: 10
            anchors.rightMargin: 10
            spacing: 8

            // ------------------------------------------------ track strips
            // Track strips, then bus strips, then "Add bus"; scrolls sideways.
            Flickable {
                id: strips
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.minimumWidth: mixer.stripWidth
                contentWidth: stripRow.width
                contentHeight: height
                flickableDirection: Flickable.HorizontalFlick
                boundsBehavior: Flickable.StopAtBounds
                clip: true
                ScrollBar.horizontal: ScrollBar { policy: ScrollBar.AsNeeded }

                Row {
                    id: stripRow
                    height: strips.height
                    spacing: 2
                    Repeater {
                        model: mixer.tracks
                        delegate: Strip {
                            required property var modelData
                            required property int index
                            track: modelData
                            level: mixer.meters[index] || [0, 0]
                        }
                    }
                    Rectangle {
                        visible: mixer.tracks.length > 0
                        width: 1; height: parent.height - 12
                        anchors.verticalCenter: parent.verticalCenter
                        color: Theme.line
                    }
                    Repeater {
                        model: mixer.buses
                        delegate: BusStrip {
                            required property var modelData
                            required property int index
                            bus: modelData
                            level: mixer.meters[mixer.tracks.length + index] || [0, 0]
                        }
                    }
                    // Add a bus
                    AbstractButton {
                        id: addBus
                        width: mixer.stripWidth
                        height: parent.height - 12
                        hoverEnabled: true
                        onClicked: mixer.app.addBus()
                        background: Rectangle {
                            radius: Theme.radiusMd
                            color: addBus.hovered ? Theme.tint(Theme.hover, 0.5) : "transparent"
                            border.color: Theme.tint(Theme.lineStrong, addBus.hovered ? 1 : 0.6)
                        }
                        contentItem: Column {
                            spacing: 6
                            topPadding: (addBus.height - 50) / 2
                            Icon { anchors.horizontalCenter: parent.horizontalCenter; glyph: Icons.plus; size: 16; color: Theme.textDim }
                            DText { anchors.horizontalCenter: parent.horizontalCenter; text: "Bus"; dim: true; font.pixelSize: Theme.textXs }
                        }
                        DToolTip {
                            visible: addBus.hovered
                            text: "Add a bus: a shared channel that tracks send to, for one reverb or delay used by many tracks"
                        }
                    }
                }
            }

            Rectangle { Layout.preferredWidth: 1; Layout.fillHeight: true; color: Theme.line }

            // ------------------------------------------------ master
            Rectangle {
                Layout.preferredWidth: mixer.masterWidth
                Layout.maximumWidth: mixer.masterWidth
                Layout.fillHeight: true
                radius: Theme.radiusMd
                color: mixer.chainType === "master" ? Theme.tint(Theme.accent, 0.12) : "transparent"
                border.color: mixer.chainType === "master" ? Theme.tint(Theme.accent, 0.45) : "transparent"
                TapHandler { onTapped: mixer.focusMaster() }
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 4
                spacing: 6
                DText {
                    Layout.alignment: Qt.AlignHCenter
                    text: "MASTER"
                    font.pixelSize: Theme.textXs
                    font.weight: Font.DemiBold
                    font.letterSpacing: 1
                }
                InsertTags {
                    Layout.fillWidth: true
                    inserts: mixer.masterInserts
                    onPicked: id => { mixer.focusMaster(); mixer.openInsert(id) }
                }
                // Clip light: lights up when the mix goes over 0 dB; click to reset.
                AbstractButton {
                    Layout.alignment: Qt.AlignHCenter
                    implicitWidth: 52; implicitHeight: 18
                    hoverEnabled: true
                    onClicked: mixer.app.clearClip()
                    background: Rectangle {
                        radius: Theme.radiusSm
                        color: mixer.app.masterClip ? Theme.danger : Theme.sunken
                        border.color: mixer.app.masterClip ? Theme.danger : Theme.line
                    }
                    contentItem: DText {
                        text: "CLIP"
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                        font.pixelSize: 9
                        font.weight: Font.DemiBold
                        color: mixer.app.masterClip ? "#FFFFFF" : Theme.textFaint
                    }
                    DToolTip {
                        visible: parent.hovered
                        text: mixer.app.masterClip ? "The mix went over 0 dB. Turn something down, then click to reset."
                                                   : "Lights up if the mix goes over 0 dB"
                    }
                }
                VFader {
                    Layout.alignment: Qt.AlignHCenter
                    Layout.fillHeight: true
                    maxDb: 0
                    db: mixer.toDb(mixer.app.volume)
                    leftLevel: mixer.app.meterL
                    rightLevel: mixer.app.meterR
                    onMoved: (d, done) => mixer.app.setVolumeLevel(d <= -60 ? 0 : Math.pow(10, d / 20))
                }
                DText {
                    Layout.alignment: Qt.AlignHCenter
                    text: mixer.dbText(mixer.toDb(mixer.app.volume)) + " dB"
                    font.family: Theme.mono
                    font.pixelSize: Theme.textXs
                    dim: true
                }
            }
            }

            Rectangle { Layout.preferredWidth: 1; Layout.fillHeight: true; color: Theme.line }

            // ------------------------------------------------ effects
            EffectsPanel {
                id: effects
                onEditRequested: id => mixer.effectEditRequested(id, chainName, tint)
                onMasteringRequested: mixer.masteringRequested()
                Layout.preferredWidth: mixer.effectsWidth
                Layout.maximumWidth: mixer.effectsWidth
                Layout.fillWidth: false
                Layout.fillHeight: true
                app: mixer.app
                chainType: mixer.chainType
                chainId: mixer.chainType === "track" ? mixer.app.selectedTrack : mixer.chainId
                chainName: mixer.chainType === "master" ? "Master · the whole mix"
                         : mixer.chainType === "bus" ? (mixer.focusedBus ? mixer.focusedBus.name + " · bus" : "")
                         : (mixer.selectedTrack ? mixer.selectedTrack.name + " · track" : "")
                inserts: mixer.chainType === "master" ? mixer.masterInserts
                       : mixer.chainType === "bus" ? (mixer.focusedBus ? mixer.focusedBus.inserts : [])
                       : (mixer.selectedTrack ? mixer.selectedTrack.inserts : [])
                tint: mixer.chainType === "bus" && mixer.focusedBus ? Theme.trackColor(mixer.focusedBus.color)
                    : mixer.chainType === "track" && mixer.selectedTrack ? Theme.trackColor(mixer.selectedTrack.color)
                    : Theme.accent
            }
        }
    }

    // One track's channel strip.
    component Strip: Rectangle {
        id: strip
        property var track
        property var level: [0, 0]
        readonly property color tint: Theme.trackColor(track.color)
        readonly property bool selected: mixer.chainType === "track" && mixer.app.selectedTrack === track.id
        width: mixer.stripWidth
        height: strips.height - 12
        radius: Theme.radiusMd
        color: selected ? Theme.tint(tint, 0.14) : hover.hovered ? Theme.tint(Theme.hover, 0.5) : "transparent"
        border.color: selected ? Theme.tint(tint, 0.5) : "transparent"

        HoverHandler { id: hover }
        TapHandler { onTapped: mixer.focusTrack(strip.track.id) }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 6
            spacing: 6

            Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 3; radius: 1.5; color: strip.tint }
            DText {
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignHCenter
                text: strip.track.name
                elide: Text.ElideRight
                font.pixelSize: Theme.textXs
                font.weight: Font.DemiBold
                color: strip.selected ? Theme.text : Theme.textDim
            }
            InsertTags {
                Layout.fillWidth: true
                inserts: strip.track.inserts || []
                onPicked: id => { mixer.focusTrack(strip.track.id); mixer.openInsert(id) }
            }
            PanKnob {
                Layout.alignment: Qt.AlignHCenter
                value: Auto.live(strip.track, "pan", mixer.app.positionBeats, mixer.app.playing, strip.track.pan)
                tint: strip.tint
                onMoved: (v, done) => mixer.app.setTrackPan(strip.track.id, v, done)
            }
            // A send knob per bus.
            Column {
                Layout.alignment: Qt.AlignHCenter
                visible: mixer.buses.length > 0
                spacing: 3
                Repeater {
                    model: mixer.buses
                    delegate: Row {
                        id: send
                        required property var modelData
                        readonly property real own: strip.track.sends && strip.track.sends[modelData.id] !== undefined
                                                    ? strip.track.sends[modelData.id] : -60
                        readonly property real db: Auto.live(strip.track, "send:" + modelData.id, mixer.app.positionBeats, mixer.app.playing, own)
                        spacing: 3
                        SendKnob {
                            db: send.db
                            tint: Theme.trackColor(send.modelData.color)
                            label: send.modelData.name
                            onMoved: (d, done) => mixer.app.setSend(strip.track.id, send.modelData.id, d, done)
                        }
                        DText {
                            width: 30
                            anchors.verticalCenter: parent.verticalCenter
                            text: send.modelData.name
                            elide: Text.ElideRight
                            font.pixelSize: 9
                            color: send.db > -60 ? Theme.textDim : Theme.textFaint
                        }
                    }
                }
            }
            VFader {
                Layout.alignment: Qt.AlignHCenter
                Layout.fillHeight: true
                db: Auto.live(strip.track, "volume", mixer.app.positionBeats, mixer.app.playing, strip.track.volume)
                tint: strip.tint
                leftLevel: strip.level[0]
                rightLevel: strip.level[1]
                onMoved: (d, done) => mixer.app.setTrackVolume(strip.track.id, d, done)
            }
            DText {
                Layout.alignment: Qt.AlignHCenter
                text: mixer.dbText(strip.track.volume)
                font.family: Theme.mono
                font.pixelSize: Theme.textXs
                dim: true
            }
            Row {
                Layout.alignment: Qt.AlignHCenter
                spacing: 4
                MiniToggle {
                    label: "M"
                    on: strip.track.mute
                    onColor: Theme.accent
                    tip: "Mute (M): silence this track. Click again to hear it."
                    onClicked: mixer.app.toggleMute(strip.track.id)
                }
                MiniToggle {
                    label: "S"
                    on: strip.track.solo
                    onColor: Theme.loopOn
                    tip: "Solo (S): hear only the soloed tracks, to check one part on its own."
                    onClicked: mixer.app.toggleSolo(strip.track.id)
                }
            }
        }
    }

    // A bus's channel strip: its sound comes from the tracks' sends.
    component BusStrip: Rectangle {
        id: busStrip
        property var bus
        property var level: [0, 0]
        readonly property color tint: Theme.trackColor(bus.color)
        width: mixer.stripWidth
        height: strips.height - 12
        radius: Theme.radiusMd
        readonly property bool focused: mixer.chainType === "bus" && mixer.chainId === bus.id
        color: Theme.tint(tint, focused ? 0.18 : busHover.hovered ? 0.12 : 0.07)
        border.color: Theme.tint(tint, focused ? 0.6 : 0.3)

        HoverHandler { id: busHover }
        TapHandler { onTapped: mixer.focusBus(busStrip.bus.id) }
        TapHandler {
            acceptedButtons: Qt.RightButton
            onTapped: busMenu.popup()
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 6
            spacing: 6

            Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 3; radius: 1.5; color: busStrip.tint }
            Item {
                Layout.fillWidth: true
                implicitHeight: busName.implicitHeight
                DText {
                    id: busName
                    visible: !busEdit.visible
                    width: parent.width
                    horizontalAlignment: Text.AlignHCenter
                    text: busStrip.bus.name
                    elide: Text.ElideRight
                    font.pixelSize: Theme.textXs
                    font.weight: Font.DemiBold
                    MouseArea { anchors.fill: parent; onDoubleClicked: busStrip.rename() }
                }
                TextInput {
                    id: busEdit
                    visible: false
                    width: parent.width
                    horizontalAlignment: TextInput.AlignHCenter
                    font.family: Theme.font
                    font.pixelSize: Theme.textXs
                    color: Theme.text
                    selectionColor: Theme.accent
                    onAccepted: { mixer.app.renameBus(busStrip.bus.id, text); visible = false }
                    onActiveFocusChanged: if (!activeFocus) visible = false
                    Keys.onEscapePressed: visible = false
                }
            }
            DText {
                Layout.alignment: Qt.AlignHCenter
                text: "BUS"
                font.pixelSize: 9
                font.letterSpacing: 1
                color: busStrip.tint
            }
            InsertTags {
                Layout.fillWidth: true
                inserts: busStrip.bus.inserts || []
                onPicked: id => { mixer.focusBus(busStrip.bus.id); mixer.openInsert(id) }
            }
            PanKnob {
                Layout.alignment: Qt.AlignHCenter
                value: busStrip.bus.pan
                tint: busStrip.tint
                onMoved: (v, done) => mixer.app.setBusPan(busStrip.bus.id, v, done)
            }
            VFader {
                Layout.alignment: Qt.AlignHCenter
                Layout.fillHeight: true
                db: busStrip.bus.volume
                tint: busStrip.tint
                leftLevel: busStrip.level[0]
                rightLevel: busStrip.level[1]
                onMoved: (d, done) => mixer.app.setBusVolume(busStrip.bus.id, d, done)
            }
            DText {
                Layout.alignment: Qt.AlignHCenter
                text: mixer.dbText(busStrip.bus.volume)
                font.family: Theme.mono
                font.pixelSize: Theme.textXs
                dim: true
            }
            MiniToggle {
                Layout.alignment: Qt.AlignHCenter
                label: "M"
                on: busStrip.bus.mute
                onColor: Theme.accent
                tip: "Mute the bus"
                onClicked: mixer.app.toggleBusMute(busStrip.bus.id)
            }
        }

        function rename() {
            busEdit.text = bus.name
            busEdit.visible = true
            busEdit.forceActiveFocus()
            busEdit.selectAll()
        }

        MenuPopup {
            id: busMenu
            function popup() { x = 6; y = 40; open() }
            items: [
                { label: "Rename", action: () => busStrip.rename() },
                { separator: true },
                { label: "Delete Bus", danger: true, action: () => mixer.app.removeBus(busStrip.bus.id) }
            ]
        }
    }

    // Small tags for a strip's effects (EQ, COMP, VERB…); click one to open it.
    component InsertTags: Column {
        id: tags
        property var inserts: []
        signal picked(int id)
        spacing: 2
        readonly property var shortNames: ({ eq: "EQ", compressor: "COMP", reverb: "VERB", delay: "DLY", limiter: "LIM", chorus: "CHOR", flanger: "FLNG", phaser: "PHAS", distortion: "DIST", bitcrusher: "CRSH", gate: "GATE", deesser: "DESS", pitch: "TUNE" })
        Repeater {
            model: tags.inserts.slice(0, 3)
            delegate: AbstractButton {
                id: tag
                required property var modelData
                width: tags.width
                height: 15
                hoverEnabled: true
                onClicked: tags.picked(modelData.id)
                background: Rectangle {
                    radius: 3
                    color: tag.modelData.enabled ? Theme.tint(Theme.accent, tag.hovered ? 0.4 : 0.25) : Theme.tint(Theme.sunken, 0.6)
                    border.color: tag.modelData.enabled ? Theme.tint(Theme.accent, 0.5) : Theme.line
                }
                contentItem: DText {
                    text: tag.modelData.plugin ? tag.modelData.label.slice(0, 5).toUpperCase() : (tags.shortNames[tag.modelData.kind] || "FX")
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    font.pixelSize: 9
                    font.weight: Font.DemiBold
                    color: tag.modelData.enabled ? Theme.text : Theme.textFaint
                }
                DToolTip { visible: tag.hovered; text: tag.modelData.label + (tag.modelData.enabled ? "" : " (bypassed)") }
            }
        }
        DText {
            visible: tags.inserts.length > 3
            width: tags.width
            horizontalAlignment: Text.AlignHCenter
            text: "+" + (tags.inserts.length - 3)
            font.pixelSize: 9
            faint: true
        }
    }
}
