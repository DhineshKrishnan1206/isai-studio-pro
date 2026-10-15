import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Isai

// ⌘K: type to find and run any command.
//
// `provider(query)` returns commands to show: [{ title, hint?, section?,
// keywords?, glyph?, action }]. It may add commands built from the query
// itself, such as "Set tempo to 128 BPM" when the query is "128".
Item {
    id: commandPalette
    property var provider: q => []
    /// What the frosted glass blurs.
    property Item backdrop: null
    property bool shown: false
    property int current: 0

    function open() {
        search.text = ""
        current = 0
        shown = true
        search.forceActiveFocus()
    }
    function close() { shown = false }
    function setQuery(text) { search.text = text }
    function run(cmd) {
        close()
        if (cmd) Qt.callLater(cmd.action)
    }

    // Fuzzy match: every typed letter must appear in order. Matches at word
    // starts and in runs score higher. Returns -1 for no match.
    function score(text, query) {
        if (query === "") return 0
        const t = text.toLowerCase(), q = query.toLowerCase()
        let ti = 0, s = 0, run = 0
        for (let qi = 0; qi < q.length; ++qi) {
            const c = q[qi]
            if (c === " ") continue
            const found = t.indexOf(c, ti)
            if (found < 0) return -1
            const wordStart = found === 0 || t[found - 1] === " " || t[found - 1] === "/"
            run = found === ti ? run + 1 : 0
            s += 1 + (wordStart ? 3 : 0) + run * 2 - Math.min(3, found - ti) * 0.2
            ti = found + 1
        }
        return s - t.length * 0.01
    }

    readonly property var results: {
        if (!shown) return []
        const q = search.text.trim()
        const all = provider(q)
        if (q === "") return all
        return all
            .map(c => ({ cmd: c, s: c.always ? 1000 : Math.max(score(c.title, q), score(c.title + " " + (c.keywords || ""), q) - 1) }))
            .filter(r => r.s >= 0)
            .sort((a, b) => b.s - a.s)
            .map(r => r.cmd)
    }
    onResultsChanged: current = 0

    anchors.fill: parent
    visible: opacity > 0
    opacity: shown ? 1 : 0
    z: 200
    Behavior on opacity { NumberAnimation { duration: Theme.fast } }

    Rectangle {
        anchors.fill: parent
        color: Theme.tint(Theme.overlay, 0.35)
        MouseArea { anchors.fill: parent; onClicked: commandPalette.close() }
    }

    GlassPanel {
        id: panel
        backdrop: commandPalette.backdrop
        clarity: 0.3
        radius: Theme.radiusXl
        width: Math.min(620, parent.width - 48)
        height: Math.min(460, column.implicitHeight + 2)
        anchors.horizontalCenter: parent.horizontalCenter
        y: Math.max(40, parent.height * 0.14)
        scale: commandPalette.shown ? 1 : 0.97
        Behavior on scale { NumberAnimation { duration: Theme.slow; easing.type: Easing.OutCubic } }

        ColumnLayout {
            id: column
            anchors.fill: parent
            spacing: 0

            RowLayout {
                Layout.fillWidth: true
                Layout.margins: 14
                spacing: 10
                Icon { glyph: Icons.magnifyingGlass; size: 18; color: Theme.textDim }
                TextField {
                    id: search
                    Layout.fillWidth: true
                    placeholderText: "Search commands, or type 128 bpm, bar 17, 3/4…"
                    font.family: Theme.font
                    font.pixelSize: Theme.textLg
                    color: Theme.text
                    placeholderTextColor: Theme.textFaint
                    selectionColor: Theme.accent
                    background: null
                    Keys.onUpPressed: commandPalette.current = Math.max(0, commandPalette.current - 1)
                    Keys.onDownPressed: commandPalette.current = Math.min(commandPalette.results.length - 1, commandPalette.current + 1)
                    Keys.onReturnPressed: commandPalette.run(commandPalette.results[commandPalette.current])
                    Keys.onEnterPressed: commandPalette.run(commandPalette.results[commandPalette.current])
                    Keys.onEscapePressed: commandPalette.close()
                }
                Badge { text: "esc"; tone: "neutral" }
            }

            Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.line }

            ListView {
                id: list
                Layout.fillWidth: true
                Layout.preferredHeight: Math.min(380, contentHeight + 12)
                Layout.topMargin: 6
                Layout.bottomMargin: 6
                clip: true
                model: commandPalette.results
                currentIndex: commandPalette.current
                boundsBehavior: Flickable.StopAtBounds
                highlightMoveDuration: 0
                onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                section.property: "section"
                section.criteria: ViewSection.FullString
                section.delegate: search.text === "" ? sectionHeader : null
                Component {
                    id: sectionHeader
                    DText {
                        required property string section
                        leftPadding: 18; topPadding: 8; bottomPadding: 4
                        text: section.toUpperCase()
                        faint: true
                        font.pixelSize: Theme.textXs
                        font.weight: Font.DemiBold
                        font.letterSpacing: 1
                    }
                }
                delegate: AbstractButton {
                    id: row
                    required property var modelData
                    required property int index
                    readonly property bool active: index === commandPalette.current
                    width: list.width - 12
                    x: 6
                    height: 36
                    hoverEnabled: true
                    onHoveredChanged: if (hovered) commandPalette.current = index
                    onClicked: commandPalette.run(modelData)
                    background: Rectangle {
                        radius: Theme.radiusMd
                        color: row.active ? Theme.accentSoft : "transparent"
                    }
                    contentItem: RowLayout {
                        spacing: 10
                        Icon {
                            Layout.leftMargin: 8
                            glyph: row.modelData.glyph || Icons.command
                            size: 16
                            color: row.active ? Theme.accent : Theme.textDim
                        }
                        DText {
                            Layout.fillWidth: true
                            text: row.modelData.title
                            elide: Text.ElideRight
                            strong: row.active
                        }
                        DText {
                            Layout.rightMargin: 8
                            visible: !!row.modelData.hint
                            text: row.modelData.hint || ""
                            faint: true
                            font.pixelSize: Theme.textSm
                        }
                    }
                }
            }

            DText {
                visible: commandPalette.results.length === 0
                Layout.margins: 18
                text: "No matching commands"
                dim: true
            }
        }
    }
}
