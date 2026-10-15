import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Isai

// The effects of one chain (a track, a bus or the master), in order.
// Click an effect to open its editor window; the dot turns it on or off
// (bypass); right-click to move or remove it.
Item {
    id: panel
    property var app
    /// "track", "bus" or "master", and the track or bus id.
    property string chainType: "track"
    property int chainId: -1
    property string chainName: ""
    property var inserts: []
    property color tint: Theme.accent
    /// Open an effect in its full editor window.
    signal editRequested(int id)
    /// The master's "Mastering assistant" button.
    signal masteringRequested()

    readonly property var kinds: {
        try { return JSON.parse(app.effectKindsJson || "[]") } catch (e) { return [] }
    }
    function kind(key) { return kinds.find(k => k.key === key) }
    function openAddMenu() { addMenu.open() }
    readonly property bool hasChain: chainType === "master" || chainId >= 0

    /// Adds an effect and opens its editor.
    function add(kindKey) {
        const id = app.addInsert(chainType, chainId, kindKey)
        if (id >= 0) editRequested(id)
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 8

        RowLayout {
            spacing: 6
            Icon { glyph: Icons.magicWand; size: 14; color: Theme.textDim }
            DText { text: "Channel Effects"; strong: true; font.pixelSize: Theme.textSm }
            Item { Layout.fillWidth: true }
            DText {
                visible: panel.inserts.length > 0
                text: panel.inserts.length
                faint: true
                font.family: Theme.mono
                font.pixelSize: Theme.textXs
            }
        }
        DText {
            Layout.fillWidth: true
            text: panel.hasChain ? panel.chainName : "Select a track, a bus or the master"
            faint: true
            font.pixelSize: Theme.textXs
            elide: Text.ElideRight
        }

        Flickable {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentHeight: column.implicitHeight
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

            ColumnLayout {
                id: column
                width: list.width
                spacing: 4

                Repeater {
                    model: panel.inserts
                    delegate: InsertRow {
                        required property var modelData
                        required property int index
                        Layout.fillWidth: true
                        insert: modelData
                        position: index
                    }
                }

                // Add an effect
                AbstractButton {
                    id: addButton
                    Layout.fillWidth: true
                    implicitHeight: 32
                    enabled: panel.hasChain
                    hoverEnabled: true
                    onClicked: addMenu.open()
                    background: Rectangle {
                        radius: Theme.radiusMd
                        color: addButton.hovered ? Theme.tint(Theme.hover, 0.6) : "transparent"
                        border.color: Theme.tint(Theme.lineStrong, addButton.hovered ? 1 : 0.6)
                    }
                    contentItem: Row {
                        spacing: 6
                        leftPadding: (addButton.width - implicitWidth) / 2
                        Icon { anchors.verticalCenter: parent.verticalCenter; glyph: Icons.plus; size: 13; color: Theme.textDim }
                        DText { anchors.verticalCenter: parent.verticalCenter; text: "Add effect"; dim: true; font.pixelSize: Theme.textSm }
                    }
                    MenuPopup {
                        id: addMenu
                        y: addButton.height + 4
                        width: Math.max(addButton.width, 200)
                        // Grouped by what they do; anything not listed goes at the end.
                        items: {
                            const groups = [["EQ & DYNAMICS", ["eq", "compressor", "limiter", "gate"]],
                                            ["VOCALS", ["pitch", "deesser"]],
                                            ["SPACE", ["reverb", "delay"]],
                                            ["MOVEMENT", ["chorus", "flanger", "phaser"]],
                                            ["CHARACTER", ["distortion", "bitcrusher"]]]
                            const out = []
                            const used = {}
                            for (const [title, keys] of groups) {
                                const found = keys.map(key => panel.kind(key)).filter(k => !!k)
                                if (!found.length) continue
                                out.push({ header: title })
                                for (const k of found) { used[k.key] = true; out.push({ label: k.label, action: () => panel.add(k.key) }) }
                            }
                            for (const k of panel.kinds) if (!used[k.key]) out.push({ label: k.label, action: () => panel.add(k.key) })
                            return out
                        }
                    }
                }

                DButton {
                    visible: panel.chainType === "master"
                    Layout.fillWidth: true
                    implicitHeight: 30
                    variant: "ghost"
                    glyph: Icons.magicWand
                    text: "Mastering assistant…"
                    onClicked: panel.masteringRequested()
                }

                DText {
                    visible: panel.hasChain && panel.inserts.length === 0
                    Layout.fillWidth: true
                    Layout.topMargin: 4
                    wrapMode: Text.WordWrap
                    faint: true
                    font.pixelSize: Theme.textXs
                    text: panel.chainType === "master"
                          ? "Effects here shape the whole mix. A limiter last keeps it from clipping."
                          : panel.chainType === "bus"
                            ? "Put a reverb or delay here, then turn up the bus's send knob on any track."
                            : "Effects run top to bottom, before the fader."
                }
            }
        }
    }

    // One effect in the chain: click to open its editor.
    component InsertRow: Rectangle {
        id: row
        property var insert
        property int position

        implicitHeight: 34
        radius: Theme.radiusMd
        color: header.pressed ? Theme.tint(panel.tint, 0.16) : header.containsMouse ? Theme.tint(Theme.hover, 0.6) : Theme.tint(Theme.sunken, Theme.glass ? 0.45 : 1)
        border.color: header.containsMouse ? Theme.tint(panel.tint, 0.45) : Theme.tint(Theme.lineStrong, 0.5)
        Behavior on color { ColorAnimation { duration: Theme.fast } }

        MouseArea {
            id: header
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            acceptedButtons: Qt.LeftButton | Qt.RightButton
            onClicked: m => {
                if (m.button === Qt.RightButton) rowMenu.open()
                else panel.editRequested(row.insert.id)
            }
        }
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 8
            anchors.rightMargin: 6
            spacing: 8
            // On / bypass
            AbstractButton {
                implicitWidth: 18; implicitHeight: 18
                hoverEnabled: true
                onClicked: panel.app.toggleInsert(row.insert.id)
                background: Rectangle {
                    radius: 9
                    color: "transparent"
                    border.color: row.insert.enabled ? panel.tint : Theme.lineStrong
                    border.width: 1.5
                    Rectangle {
                        anchors.centerIn: parent
                        width: 8; height: 8; radius: 4
                        color: row.insert.enabled ? panel.tint : "transparent"
                    }
                }
                DToolTip { visible: parent.hovered; text: row.insert.enabled ? "On · click to bypass" : "Bypassed · click to turn on" }
            }
            DText {
                Layout.fillWidth: true
                text: row.insert.label
                elide: Text.ElideRight
                font.pixelSize: Theme.textSm
                color: row.insert.enabled ? Theme.text : Theme.textFaint
            }
            Icon {
                glyph: Icons.sliders
                size: 13
                color: header.containsMouse ? panel.tint : Theme.textFaint
            }
            IconButton {
                implicitWidth: 20; implicitHeight: 20; iconSize: 12
                glyph: Icons.x
                tip: "Remove"
                onClicked: panel.app.removeInsert(row.insert.id)
            }
        }
        DToolTip { visible: header.containsMouse; delay: 600; text: "Click to open " + row.insert.label + " · right-click for more" }

        MenuPopup {
            id: rowMenu
            y: 30
            width: 180
            items: [
                { label: "Open Editor", action: () => panel.editRequested(row.insert.id) },
                { label: "Move Up", enabled: row.position > 0, action: () => panel.app.moveInsert(row.insert.id, row.position - 1) },
                { label: "Move Down", enabled: row.position < panel.inserts.length - 1, action: () => panel.app.moveInsert(row.insert.id, row.position + 1) },
                { label: row.insert.enabled ? "Bypass" : "Turn On", action: () => panel.app.toggleInsert(row.insert.id) },
                { separator: true },
                { label: "Remove", danger: true, action: () => panel.app.removeInsert(row.insert.id) }
            ]
        }
    }
}
