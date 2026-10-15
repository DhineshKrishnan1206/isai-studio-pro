import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Effects
import QtQuick.Layouts
import Isai

// A dropdown menu. items: [{ label, shortcut?, enabled?, danger?, action }]
// or { separator: true } or { header: "TITLE" }.
Popup {
    id: menu
    property var items: []
    /// Taller menus (a long list of plugins) scroll.
    property real maxHeight: 520
    signal triggered()

    padding: 5
    width: 260
    modal: false
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    // Opens below its button by default (y), but flips above it when the
    // window has no room below, e.g. for buttons low in the docked mixer.
    property real preferredY: NaN
    onAboutToShow: {
        // Lay the items out now, so the size (and the flip below) is right on
        // the first open; otherwise the menu could open cut short.
        column.forceLayout()
        if (isNaN(preferredY)) preferredY = y
        y = preferredY
        if (!parent || preferredY <= 0) return
        const win = parent.Window.window
        if (!win) return
        const top = parent.mapToItem(null, 0, 0).y
        const below = win.height - (top + preferredY) - 8
        const above = top - 8
        if (implicitHeight > below && above > below)
            y = -Math.min(implicitHeight, above) - 4
    }

    background: GlassPanel { backdrop: Theme.backdrop; radius: Theme.radiusLg; clarity: 0.3 }
    enter: Transition { NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.fast } }
    exit: Transition { NumberAnimation { property: "opacity"; to: 0; duration: Theme.fast } }

    contentItem: Flickable {
        implicitWidth: menu.availableWidth
        implicitHeight: Math.min(column.implicitHeight, menu.maxHeight)
        contentHeight: column.implicitHeight
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        interactive: contentHeight > height
        ScrollBar.vertical: ScrollBar { policy: column.implicitHeight > menu.maxHeight ? ScrollBar.AsNeeded : ScrollBar.AlwaysOff }
      Column {
        id: column
        width: parent.width
        spacing: 1
        Repeater {
            model: menu.items
            delegate: Loader {
                required property var modelData
                width: menu.availableWidth
                sourceComponent: modelData.separator ? separator : modelData.header ? header : entry
                Component {
                    id: separator
                    Item { height: 9; Rectangle { anchors.centerIn: parent; width: parent.width - 8; height: 1; color: Theme.line } }
                }
                Component {
                    id: header
                    DText {
                        leftPadding: 10; topPadding: 6; bottomPadding: 2
                        text: modelData.header
                        faint: true
                        font.pixelSize: Theme.textXs
                        font.weight: Font.DemiBold
                        font.letterSpacing: 1
                    }
                }
                Component {
                    id: entry
                    AbstractButton {
                        id: item
                        height: 30
                        enabled: modelData.enabled === undefined || modelData.enabled
                        hoverEnabled: true
                        onClicked: { menu.close(); modelData.action(); menu.triggered() }
                        background: Rectangle {
                            radius: Theme.radiusSm
                            color: item.hovered && item.enabled ? (modelData.danger ? Theme.tint(Theme.danger, 0.15) : Theme.hover) : "transparent"
                        }
                        contentItem: RowLayout {
                            spacing: 8
                            DText {
                                Layout.fillWidth: true
                                leftPadding: 6
                                text: modelData.label
                                elide: Text.ElideRight
                                color: !item.enabled ? Theme.textFaint : modelData.danger ? Theme.danger : Theme.text
                            }
                            DText {
                                rightPadding: 6
                                visible: !!modelData.shortcut
                                text: modelData.shortcut || ""
                                faint: true
                                font.pixelSize: Theme.textSm
                            }
                        }
                    }
                }
            }
        }
      }
    }
}
