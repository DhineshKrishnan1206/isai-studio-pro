import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Dialogs
import QtQuick.Layouts
import Isai

// Export (bounce): the mix or one file per track, as WAV or FLAC.
Item {
    id: dialog
    /// What the frosted glass blurs: the page behind the dialog.
    property Item backdrop: null
    property var app
    property bool shown: false

    property bool stems: false
    property bool cycleOnly: false
    property string container: "wav"     // wav | flac
    property string depth: "24"          // 16 | 24 | 32f
    property bool normalize: false

    readonly property string formatKey: container === "flac"
                                        ? "flac" + (depth === "32f" ? "24" : depth)
                                        : "wav" + depth
    readonly property bool finished: !app.exporting && app.exportProgress >= 1 && started
    property bool started: false

    function open() { started = false; shown = true }
    function close() { if (!app.exporting) shown = false }

    anchors.fill: parent
    visible: opacity > 0
    opacity: shown ? 1 : 0
    z: 120
    Behavior on opacity { NumberAnimation { duration: Theme.fast } }
    Shortcut { sequence: "Escape"; enabled: dialog.shown; onActivated: dialog.close() }

    FileDialog {
        id: mixDialog
        title: "Export mix"
        fileMode: FileDialog.SaveFile
        nameFilters: dialog.container === "flac" ? ["FLAC audio (*.flac)"] : ["WAV audio (*.wav)"]
        defaultSuffix: dialog.container
        selectedFile: currentFolder + "/" + dialog.app.projectName + "." + dialog.container
        onAccepted: { dialog.started = true; dialog.app.exportAudio(selectedFile, false, dialog.cycleOnly, dialog.formatKey, dialog.normalize) }
    }
    FolderDialog {
        id: stemsDialog
        title: "Choose a folder for the stems"
        onAccepted: { dialog.started = true; dialog.app.exportAudio(selectedFolder, true, dialog.cycleOnly, dialog.formatKey, dialog.normalize) }
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.overlay
        MouseArea { anchors.fill: parent; onClicked: dialog.close() }
    }

    GlassPanel {
        backdrop: dialog.backdrop
        clarity: 0.3
        radius: Theme.radiusXl
        anchors.centerIn: parent
        width: 520
        height: content.implicitHeight + 48
        scale: dialog.shown ? 1 : 0.96
        Behavior on scale { NumberAnimation { duration: Theme.slow; easing.type: Easing.OutCubic } }
        MouseArea { anchors.fill: parent }

        ColumnLayout {
            id: content
            anchors.fill: parent
            anchors.margins: 24
            spacing: 0

            Item {
                Layout.fillWidth: true
                Layout.bottomMargin: 6
                implicitHeight: heading.implicitHeight
                ColumnLayout {
                    id: heading
                    anchors.left: parent.left
                    anchors.right: closeButton.left
                    spacing: 2
                    DText { text: "Export"; font.pixelSize: Theme.textXl; font.weight: Font.DemiBold }
                    DText { text: "Bounce your song to an audio file you can share."; dim: true }
                }
                IconButton {
                    id: closeButton
                    anchors.right: parent.right
                    anchors.top: parent.top
                    glyph: Icons.x
                    tip: "Close (Esc)"
                    enabled: !dialog.app.exporting
                    onClicked: dialog.close()
                }
            }

            SettingRow {
                title: "What"
                description: dialog.stems ? "One file per track (muted tracks are skipped)." : "The whole mix as one stereo file."
                Segmented {
                    options: [{ value: false, label: "Mix" }, { value: true, label: "Stems" }]
                    value: dialog.stems
                    onActivated: v => dialog.stems = v
                }
            }
            SettingRow {
                title: "Range"
                description: dialog.cycleOnly ? "Only the yellow cycle region." : "From bar 1 to the end of the last region."
                Segmented {
                    options: [{ value: false, label: "Whole song" }, { value: true, label: "Cycle region" }]
                    value: dialog.cycleOnly
                    onActivated: v => dialog.cycleOnly = v
                }
            }
            SettingRow {
                title: "Format"
                description: dialog.container === "flac" ? "Lossless and about half the size of WAV." : "Plays everywhere."
                Segmented {
                    options: [{ value: "wav", label: "WAV" }, { value: "flac", label: "FLAC" }]
                    value: dialog.container
                    onActivated: v => { dialog.container = v; if (v === "flac" && dialog.depth === "32f") dialog.depth = "24" }
                }
            }
            SettingRow {
                title: "Quality"
                description: dialog.depth === "16" ? "CD quality, with dither." : dialog.depth === "24" ? "Studio quality." : "Highest quality; can't clip."
                Segmented {
                    options: dialog.container === "flac"
                             ? [{ value: "16", label: "16-bit" }, { value: "24", label: "24-bit" }]
                             : [{ value: "16", label: "16-bit" }, { value: "24", label: "24-bit" }, { value: "32f", label: "32-bit float" }]
                    value: dialog.depth
                    onActivated: v => dialog.depth = v
                }
            }
            SettingRow {
                title: "Normalize"
                description: "Make the loudest moment peak at −1 dB."
                divider: false
                Toggle { checked: dialog.normalize; onToggled: dialog.normalize = checked }
            }

            // Progress / result
            ColumnLayout {
                Layout.fillWidth: true
                Layout.topMargin: 10
                visible: dialog.app.exporting || dialog.finished
                spacing: 6
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 6
                    radius: 3
                    color: Theme.sunken
                    Rectangle {
                        width: parent.width * Math.max(0, Math.min(1, dialog.app.exportProgress))
                        height: parent.height
                        radius: 3
                        color: Theme.accent
                        Behavior on width { NumberAnimation { duration: Theme.fast } }
                    }
                }
                DText {
                    Layout.fillWidth: true
                    text: dialog.app.exporting ? "Exporting… " + Math.round(dialog.app.exportProgress * 100) + "%" : dialog.app.status
                    dim: true
                    wrapMode: Text.WordWrap
                }
            }

            RowLayout {
                Layout.topMargin: 16
                spacing: 8
                DButton {
                    visible: dialog.finished && dialog.app.exportFolder !== ""
                    glyph: Icons.folderOpen
                    text: Qt.platform.os === "osx" ? "Show in Finder" : "Open folder"
                    onClicked: Qt.openUrlExternally("file://" + dialog.app.exportFolder)
                }
                Item { Layout.fillWidth: true }
                DButton {
                    visible: dialog.app.exporting
                    variant: "ghost"
                    text: "Cancel"
                    onClicked: dialog.app.cancelExport()
                }
                DButton {
                    variant: "primary"
                    enabled: !dialog.app.exporting
                    glyph: Icons.downloadSimple
                    text: dialog.finished ? "Export again…" : "Export…"
                    onClicked: dialog.stems ? stemsDialog.open() : mixDialog.open()
                }
            }
        }
    }
}
