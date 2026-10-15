import QtQuick
import Isai

// Output device choice, shared by setup and Settings. Index 0 is "System default".
Dropdown {
    id: picker
    property var app

    model: {
        const names = ["System default"]
        for (let i = 0; i < app.deviceNames.length; ++i)
            names.push(app.deviceNames[i])
        return names
    }
    currentIndex: {
        if (app.deviceId === "") return 0
        for (let i = 0; i < app.deviceIds.length; ++i)
            if (app.deviceIds[i] === app.deviceId) return i + 1
        return 0
    }
    onActivated: index => app.setText("deviceId", index === 0 ? "" : app.deviceIds[index - 1])
}
