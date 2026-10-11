function sin_wave(t, freq)
    return math.sin(t * 2 * math.pi * freq)
end

return {
    at = function(t, channel) return sin_wave(t, 440) end,
    duration = 1.0,
}
