import QtQuick
import QtQuick.Shapes
import Isai

// A soft circular glow drawn with a radial gradient. Much cheaper than
// blurring, so it is fine on low-spec machines too.
Shape {
    id: glow
    property color color: Theme.accent
    property real strength: 0.25
    property real size: 600

    width: size
    height: size
    preferredRendererType: Shape.CurveRenderer

    ShapePath {
        strokeColor: "transparent"
        fillGradient: RadialGradient {
            centerX: glow.size / 2; centerY: glow.size / 2
            centerRadius: glow.size / 2
            focalX: centerX; focalY: centerY
            GradientStop { position: 0.0; color: Theme.tint(glow.color, glow.strength) }
            GradientStop { position: 0.45; color: Theme.tint(glow.color, glow.strength * 0.45) }
            GradientStop { position: 1.0; color: Theme.tint(glow.color, 0) }
        }
        PathRectangle { x: 0; y: 0; width: glow.size; height: glow.size }
    }
}
