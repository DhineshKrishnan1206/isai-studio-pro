import QtCore
import QtQuick
import QtQuick.Controls.Basic
import Qt.labs.folderlistmodel
import Isai

// The sound browser (B): a panel on the left for finding sounds on your
// computer.
//   • Places: Music, Downloads, Desktop, Documents, your project's folder,
//     and folders you star (★ adds the folder you're in).
//   • Click a sound to hear it (In time waits for the next bar while the
//     song plays); double-click to put it on a new track at the playhead,
//     or drag it onto a track. Projects open with a double-click.
//   • Type to search this folder and every folder inside it.
Item {
    id: browser
    property var app
    property bool shown: false
    property Item backdrop: null
    signal closeRequested()
    /// A project file was double-clicked (the studio asks about unsaved work first).
    signal projectRequested(string url)

    width: 320
    visible: opacity > 0
    opacity: shown ? 1 : 0
    x: shown ? 12 : -width
    Behavior on opacity { NumberAnimation { duration: Theme.fast } }
    Behavior on x { NumberAnimation { duration: Theme.slow; easing.type: Easing.OutCubic } }
    z: 60

    Settings {
        id: prefs
        category: "browser"
        property string folder: ""
        property var favourites: []
        property bool inTime: true
        property bool autoPlay: true
    }

    function urlOf(path) { return path.startsWith("file:") ? path : "file://" + path }
    function pathOf(url) { return decodeURIComponent(String(url).replace(/^file:\/\//, "")) }
    function nameOf(url) { const p = pathOf(url).replace(/\/$/, ""); return p.substring(p.lastIndexOf("/") + 1) || "/" }
    readonly property string musicDir: StandardPaths.writableLocation(StandardPaths.MusicLocation)
    property string folder: ""
    Component.onCompleted: folder = prefs.folder !== "" ? prefs.folder : String(musicDir)
    onFolderChanged: if (folder !== "") prefs.folder = folder
    readonly property string projectDir: app.projectPath !== "" ? urlOf(app.projectPath.substring(0, app.projectPath.lastIndexOf("/"))) : ""
    readonly property var places: {
        const list = [
            { name: "Music", url: String(StandardPaths.writableLocation(StandardPaths.MusicLocation)) },
            { name: "Downloads", url: String(StandardPaths.writableLocation(StandardPaths.DownloadLocation)) },
            { name: "Desktop", url: String(StandardPaths.writableLocation(StandardPaths.DesktopLocation)) },
            { name: "Documents", url: String(StandardPaths.writableLocation(StandardPaths.DocumentsLocation)) }
        ]
        if (projectDir !== "") list.push({ name: "This project", url: projectDir })
        for (const f of prefs.favourites || []) list.push({ name: "★ " + nameOf(f), url: f, favourite: true })
        return list
    }
    readonly property bool starred: (prefs.favourites || []).indexOf(folder) >= 0
    function toggleStar() {
        const list = (prefs.favourites || []).slice()
        const i = list.indexOf(folder)
        if (i >= 0) list.splice(i, 1); else list.push(folder)
        prefs.favourites = list
    }

    // ------------------------------------------------------------ playing
    property string playing: ""
    function preview(url) {
        playing = url
        app.previewFile(url, prefs.inTime)
    }
    function stop() { playing = ""; app.stopPreview() }
    Connections {
        target: browser.app
        function onPreviewingChanged() { if (!browser.app.previewing) browser.playing = "" }
    }
    onShownChanged: if (!shown) stop()
    function open(url, isDir, kind) {
        if (isDir) { folder = url; query.text = ""; return }
        if (kind === "project") { projectRequested(url); return }
        app.importAt(url, -1, 0)
    }

    // ------------------------------------------------------------ search
    property var results: []
    readonly property bool searching: query.text.trim() !== ""
    Timer {
        id: searchTimer
        interval: 250
        onTriggered: {
            try { browser.results = JSON.parse(browser.app.findSounds(browser.folder, query.text.trim()) || "[]") } catch (e) { browser.results = [] }
        }
    }

    FolderListModel {
        id: files
        folder: browser.folder
        showDirsFirst: true
        showHidden: false
        caseSensitive: false
        nameFilters: ["*.wav", "*.flac", "*.mp3", "*.ogg", "*.m4a", "*.aac", "*.aif", "*.aiff", "*.caf", "*.isai",
                      "*.WAV", "*.FLAC", "*.MP3", "*.AIF", "*.AIFF"]
    }

    // ------------------------------------------------------------ body
    GlassPanel {
        anchors.fill: parent
        backdrop: browser.backdrop
        MouseArea { anchors.fill: parent; acceptedButtons: Qt.AllButtons; onWheel: w => w.accepted = true }
    }

    Column {
        id: top
        x: 12; y: 10
        width: parent.width - 24
        spacing: 8
        Row {
            width: parent.width
            spacing: 8
            Icon { glyph: Icons.folderOpen; size: 15; color: Theme.accent; anchors.verticalCenter: parent.verticalCenter }
            DText { text: "Browser"; strong: true; anchors.verticalCenter: parent.verticalCenter }
            Item { width: parent.width - 196; height: 1 }
            IconButton {
                implicitWidth: 26; implicitHeight: 26; iconSize: 13
                glyph: Icons.x
                tip: "Close the browser (B)"
                onClicked: browser.closeRequested()
            }
        }
        // Places
        Flow {
            width: parent.width
            spacing: 4
            Repeater {
                model: browser.places
                delegate: AbstractButton {
                    id: place
                    required property var modelData
                    readonly property bool here: browser.folder === modelData.url
                    implicitWidth: placeText.implicitWidth + 16
                    implicitHeight: 24
                    hoverEnabled: true
                    onClicked: { browser.folder = modelData.url; query.text = "" }
                    background: Rectangle {
                        radius: 12
                        color: place.here ? Theme.accentSoft : place.hovered ? Theme.hover : Theme.tint(Theme.sunken, 0.6)
                        border.color: place.here ? Theme.tint(Theme.accent, 0.5) : Theme.line
                    }
                    contentItem: Item {
                        DText { id: placeText; anchors.centerIn: parent; text: place.modelData.name; font.pixelSize: Theme.textXs; color: place.here ? Theme.text : Theme.textDim }
                    }
                }
            }
        }
        // Where we are
        Row {
            width: parent.width
            spacing: 6
            IconButton {
                implicitWidth: 26; implicitHeight: 26; iconSize: 12
                glyph: Icons.arrowLeft
                tip: "Up one folder"
                enabled: String(files.parentFolder) !== ""
                onClicked: { browser.folder = String(files.parentFolder); query.text = "" }
            }
            DText {
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width - 70
                elide: Text.ElideMiddle
                text: browser.nameOf(browser.folder)
                font.pixelSize: Theme.textSm
                strong: true
            }
            IconButton {
                implicitWidth: 26; implicitHeight: 26; iconSize: 12
                glyph: Icons.sparkle
                active: browser.starred
                tip: browser.starred ? "Remove this folder from your places" : "Add this folder to your places"
                onClicked: browser.toggleStar()
            }
        }
        // Search
        Rectangle {
            width: parent.width
            height: 30
            radius: Theme.radiusMd
            color: Theme.tint(Theme.sunken, 0.8)
            border.color: query.activeFocus ? Theme.accent : Theme.line
            Icon { x: 9; anchors.verticalCenter: parent.verticalCenter; glyph: Icons.magnifyingGlass; size: 13; color: Theme.textFaint }
            TextInput {
                id: query
                x: 28
                width: parent.width - 36
                anchors.verticalCenter: parent.verticalCenter
                font.family: Theme.font
                font.pixelSize: Theme.textSm
                color: Theme.text
                clip: true
                onTextChanged: searchTimer.restart()
                Keys.onEscapePressed: { text = ""; focus = false }
            }
            DText {
                visible: query.text === ""
                x: 28; anchors.verticalCenter: parent.verticalCenter
                text: "Search here and in every folder inside"
                font.pixelSize: Theme.textSm
                faint: true
            }
        }
    }

    // ------------------------------------------------------------ the list
    ListView {
        id: list
        x: 6
        y: top.y + top.height + 8
        width: parent.width - 12
        height: parent.height - y - footer.height - 14
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
        model: browser.searching ? browser.results : files
        delegate: Item {
            id: row
            required property int index
            // FolderListModel roles, or a search result.
            required property var model
            readonly property string url: browser.searching ? browser.urlOf(model.modelData.path) : String(model.fileUrl)
            readonly property string name: browser.searching ? model.modelData.name : model.fileName
            readonly property bool isDir: !browser.searching && model.fileIsDir
            readonly property string kind: isDir ? "folder" : name.toLowerCase().endsWith(".isai") ? "project" : "audio"
            readonly property string sub: browser.searching ? model.modelData.folder : ""
            readonly property bool playingThis: browser.playing === url
            width: list.width
            height: sub !== "" ? 40 : 30
            Rectangle {
                anchors.fill: parent
                anchors.margins: 1
                radius: Theme.radiusSm
                color: row.playingThis ? Theme.accentSoft : rowArea.containsMouse ? Theme.hover : "transparent"
            }
            Icon {
                x: 10; anchors.verticalCenter: parent.verticalCenter
                glyph: row.isDir ? Icons.folderOpen : row.kind === "project" ? Icons.musicNotes : row.playingThis ? Icons.speakerHigh : Icons.waveform
                size: 13
                color: row.isDir ? Theme.textDim : row.playingThis ? Theme.accent : Theme.textFaint
            }
            Column {
                x: 32
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width - 70
                DText { width: parent.width; elide: Text.ElideMiddle; text: row.name; font.pixelSize: Theme.textSm }
                DText { visible: row.sub !== ""; width: parent.width; elide: Text.ElideMiddle; text: row.sub; font.pixelSize: Theme.textXs; faint: true }
            }
            // Hidden: what a drag carries (a file URL, like a drag from the Finder).
            Item {
                id: carry
                Drag.dragType: Drag.Automatic
                Drag.supportedActions: Qt.CopyAction
                Drag.mimeData: ({ "text/uri-list": row.url })
            }
            MouseArea {
                id: rowArea
                anchors.fill: parent
                hoverEnabled: true
                drag.target: row.isDir ? null : carry
                onPressed: if (!row.isDir && row.kind === "audio" && prefs.autoPlay) browser.preview(row.url)
                onDoubleClicked: browser.open(row.url, row.isDir, row.kind)
                onClicked: if (row.isDir) browser.open(row.url, true, "folder")
                drag.onActiveChanged: if (drag.active) carry.Drag.active = true; else carry.Drag.active = false
            }
            IconButton {
                visible: row.kind === "audio" && (rowArea.containsMouse || row.playingThis)
                anchors.right: parent.right; anchors.rightMargin: 4
                anchors.verticalCenter: parent.verticalCenter
                implicitWidth: 24; implicitHeight: 24; iconSize: 11
                glyph: row.playingThis ? Icons.stop : Icons.play
                tip: row.playingThis ? "Stop" : "Hear it"
                onClicked: row.playingThis ? browser.stop() : browser.preview(row.url)
            }
        }
        DText {
            visible: list.count === 0
            anchors.centerIn: parent
            width: list.width - 40
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            text: browser.searching ? "No sounds called that here." : "No sounds in this folder. Pick a place above, or open a folder."
            faint: true
            font.pixelSize: Theme.textSm
        }
    }

    // ------------------------------------------------------------ footer
    Row {
        id: footer
        x: 12
        anchors.bottom: parent.bottom; anchors.bottomMargin: 10
        width: parent.width - 24
        height: 26
        spacing: 10
        Toggle { id: autoToggle; checked: prefs.autoPlay; onToggled: prefs.autoPlay = checked; anchors.verticalCenter: parent.verticalCenter }
        DText { text: "Play on click"; font.pixelSize: Theme.textXs; dim: true; anchors.verticalCenter: parent.verticalCenter }
        Toggle { checked: prefs.inTime; onToggled: prefs.inTime = checked; anchors.verticalCenter: parent.verticalCenter }
        DText { text: "In time"; font.pixelSize: Theme.textXs; dim: true; anchors.verticalCenter: parent.verticalCenter }
    }
}
