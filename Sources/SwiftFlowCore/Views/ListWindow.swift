struct ListWindow: Equatable {

    let first: Int

    let count: Int

    let leadingPad: Float

    let trailingPad: Float

    var last: Int { first + count }

    static let overscan: Float = 180

    static let firstFrameRows = 40

    init(
        rowCount: Int,
        offset: Float,
        viewportLength: Float,
        overscan: Float = ListWindow.overscan,
        height: (Int) -> Float
    ) {
        guard rowCount > 0 else {
            self.first = 0
            self.count = 0
            self.leadingPad = 0
            self.trailingPad = 0
            return
        }

        guard viewportLength > 0 else {
            let count = min(rowCount, ListWindow.firstFrameRows)
            var trailing: Float = 0
            for i in count..<rowCount { trailing += max(0, height(i)) }
            self.first = 0
            self.count = count
            self.leadingPad = 0
            self.trailingPad = trailing
            return
        }

        var content: Float = 0
        for i in 0..<rowCount { content += max(0, height(i)) }

        let maxOffset = max(0, content - viewportLength)
        let visibleTop = min(max(0, offset), maxOffset)
        let top = max(0, visibleTop - overscan)
        let bottom = min(content, visibleTop + viewportLength + overscan)

        var firstIndex = 0
        var leading: Float = 0
        var cursor: Float = 0
        var index = 0

        while index < rowCount {
            let h = max(0, height(index))
            if cursor + h > top { break }
            cursor += h
            leading = cursor
            index += 1
            firstIndex = index
        }

        var built = 0
        while index < rowCount, cursor < bottom {
            cursor += max(0, height(index))
            index += 1
            built += 1
        }

        var trailing: Float = 0
        for i in index..<rowCount { trailing += max(0, height(i)) }

        self.first = firstIndex
        self.count = built
        self.leadingPad = leading
        self.trailingPad = trailing
    }
}
