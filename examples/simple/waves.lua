return {
    sin = function(freq)
        return function(t)
            return math.sin(t * 2 * math.pi * freq)
        end
    end,
    cos = function(t, freq)
        return function(t)
            return math.cos(t * 2 * math.pi * freq)
        end
    end,
}
