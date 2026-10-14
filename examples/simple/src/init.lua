local waves = require("example.waves")

local wave = waves.sin(440)

return {
    at = function(t, channel) return wave(t + channel) end,
    duration = 1.0,
    channels = 2,
}
