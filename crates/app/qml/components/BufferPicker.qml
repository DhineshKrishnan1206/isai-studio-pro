import QtQuick
import Isai

// Audio buffer size choice, shared by setup and Settings.
Segmented {
    property var app
    minItemWidth: 54
    options: [
        { value: 128, label: "128" },
        { value: 256, label: "256" },
        { value: 512, label: "512" },
        { value: 1024, label: "1024" },
        { value: 2048, label: "2048" }
    ]
    value: app.bufferSize
    onActivated: v => app.setInt("bufferSize", v)

    function latencyText() {
        const rate = app.sampleRate > 0 ? app.sampleRate : 48000
        return (app.bufferSize / rate * 1000).toFixed(1) + " ms delay"
    }
}
