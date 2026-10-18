import QtQuick
import QtQuick.Controls.Basic

// The menu bar: the one at the top of the screen on macOS, the strip under the
// title bar on Windows and Linux. Every item is a command from ⌘K, grouped by
// section, so the menus and the palette never disagree.
MenuBar {
    id: bar
    /// The studio page that owns the commands.
    required property var studio

    // Commands that work without an open project.
    function worksWithoutProject(title) {
        return title.indexOf("New project") === 0 || title.indexOf("Open") === 0
            || title.indexOf("Settings") === 0 || title.indexOf("Dark theme") === 0
            || title.indexOf("Light theme") === 0 || title.indexOf("Follow system") === 0
            || title.indexOf("Studio style") === 0 || title.indexOf("Midnight style") === 0
            || title.indexOf("Vivid style") === 0
    }

    function commandsFor(sections) {
        return bar.studio.commands("").filter(c => sections.indexOf(c.section) >= 0)
    }

    Menu {
        title: "File"
        Repeater {
            model: bar.commandsFor(["File"])
            delegate: MenuItem {
                required property var modelData
                text: modelData.title
                enabled: bar.studio.app.projectOpen || bar.worksWithoutProject(modelData.title)
                onTriggered: modelData.action()
            }
        }
    }
    Menu {
        title: "Edit"
        Repeater {
            model: bar.commandsFor(["Edit"])
            delegate: MenuItem {
                required property var modelData
                text: modelData.title
                enabled: bar.studio.app.projectOpen || bar.worksWithoutProject(modelData.title)
                onTriggered: modelData.action()
            }
        }
    }
    Menu {
        title: "View"
        Repeater {
            model: bar.commandsFor(["View", "Appearance"])
            delegate: MenuItem {
                required property var modelData
                text: modelData.title
                enabled: bar.studio.app.projectOpen || bar.worksWithoutProject(modelData.title)
                onTriggered: modelData.action()
            }
        }
    }
    Menu {
        title: "Transport"
        Repeater {
            model: bar.commandsFor(["Transport", "Markers"])
            delegate: MenuItem {
                required property var modelData
                text: modelData.title
                enabled: bar.studio.app.projectOpen || bar.worksWithoutProject(modelData.title)
                onTriggered: modelData.action()
            }
        }
    }
    Menu {
        title: "Track"
        Repeater {
            model: bar.commandsFor(["Tracks"])
            delegate: MenuItem {
                required property var modelData
                text: modelData.title
                enabled: bar.studio.app.projectOpen || bar.worksWithoutProject(modelData.title)
                onTriggered: modelData.action()
            }
            MenuSeparator {}
            MenuItem {
                text: "New folder with selected track"
                enabled: bar.studio.app.projectOpen && bar.studio.app.selectedTrack >= 0
                onTriggered: bar.studio.app.groupTrack(bar.studio.app.selectedTrack)
            }
            MenuItem {
                text: "Take selected track out of its folder"
                enabled: bar.studio.app.projectOpen && bar.studio.app.selectedTrack >= 0
                onTriggered: bar.studio.app.ungroupTrack(bar.studio.app.selectedTrack)
            }
        }
    }
    Menu {
        title: "Tools"
        Repeater {
            model: bar.commandsFor(["Tools"])
            delegate: MenuItem {
                required property var modelData
                text: modelData.title
                enabled: bar.studio.app.projectOpen || bar.worksWithoutProject(modelData.title)
                onTriggered: modelData.action()
            }
        }
    }

    // The lesson list, opened from the Help menu.
    LearnBrowser { id: learnBrowser; backdrop: bar.studio }

    Menu {
        title: "Help"
        MenuItem {
            text: "Learn: browse every lesson…"
            onTriggered: learnBrowser.open()
        }
    }
}
