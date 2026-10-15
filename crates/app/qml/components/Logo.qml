import QtQuick
import Isai

// Isai branding. `variant` is "mark" (the brush Isai) or "lockup" (Isai Studio Pro).
// `size` is the height in pixels. Light ink is used on dark themes and dark
// ink on light themes. The images come from assets/brand/ (see tools/brand.py).
Item {
    property string variant: "mark"
    property real size: 32

    // Taken from the image itself, so replacing the artwork needs no code change.
    readonly property real aspect: art.implicitHeight > 0 ? art.implicitWidth / art.implicitHeight
                                                          : (variant === "lockup" ? 3.6 : 1.6)

    // A wrapper, because an Image's own implicit size is its file resolution,
    // which would make layouts reserve far too much space.
    implicitHeight: size
    implicitWidth: size * aspect

    Image {
        id: art
        anchors.fill: parent
        source: "qrc:/qt/qml/Isai/assets/brand/isai-" + (parent.variant === "lockup" ? "studio-pro" : "mark")
                + (Theme.dark ? "-light.png" : "-dark.png")
        sourceSize.height: Math.min(600, Math.ceil(parent.size * 3))
        fillMode: Image.PreserveAspectFit
        smooth: true
    }
}
