pragma Singleton
import QtQuick

// Helpers for automation lanes (see `autoTargets` in each track's JSON:
// { key, label, unit, min, max, value, points: [[beat, value]…] }).
QtObject {
    /// One of a track's automation targets by key, or null.
    function target(track, key) {
        if (!track || !track.autoTargets) return null
        return track.autoTargets.find(t => t.key === key) || null
    }

    /// The lane's value at `beat`: straight lines between points, flat before
    /// the first and after the last. `fallback` when there are no points.
    function valueAt(points, beat, fallback) {
        if (!points || points.length === 0) return fallback
        if (beat <= points[0][0]) return points[0][1]
        for (let i = 1; i < points.length; ++i) {
            const [b1, v1] = points[i]
            if (beat < b1) {
                const [b0, v0] = points[i - 1]
                return b1 > b0 ? v0 + (v1 - v0) * (beat - b0) / (b1 - b0) : v0
            }
        }
        return points[points.length - 1][1]
    }

    /// What a control should show: the automated value while playing, else its own.
    function live(track, key, beat, playing, own) {
        const t = target(track, key)
        if (!playing || !t || t.points.length === 0) return own
        return valueAt(t.points, beat, own)
    }

    function format(t, v) {
        if (!t) return ""
        switch (t.unit) {
        case "dB": return v <= -60 ? "-∞ dB" : (v > 0.05 ? "+" : "") + v.toFixed(1) + " dB"
        case "pan": return Math.abs(v) < 0.01 ? "Centre" : (v < 0 ? "L" : "R") + Math.round(Math.abs(v) * 100)
        case "Hz": return v >= 1000 ? (v / 1000).toFixed(2) + " kHz" : Math.round(v) + " Hz"
        case "ms": return Math.round(v) + " ms"
        case "%": return Math.round(v) + "%"
        case ":1": return v.toFixed(1) + ":1"
        default: return v.toFixed(2)
        }
    }

    /// 0..1 height on the lane for a value, and back.
    function norm(t, v) { return t ? (v - t.min) / (t.max - t.min) : 0 }
    function denorm(t, n) { return t ? t.min + Math.max(0, Math.min(1, n)) * (t.max - t.min) : 0 }
}
