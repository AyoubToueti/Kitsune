// A grid of poster-card placeholders, shared by the My List, Browse, and
// Recommendations screens while their first page is loading. Each cell mirrors
// `PosterCard`'s shape -- a 2:3 poster block over two title lines -- at the
// cell size the caller uses, so the real grid swaps in without a shift.
//
// A plain `Grid` (not a `GridView`): the placeholder count is fixed, so there is
// nothing to scroll and no model to own.
import QtQuick

Grid {
    id: grid

    // How many placeholder cards to draw.
    property int count: 12
    // The cell footprint, matching the real screen's GridView.
    property real cellWidth: 200
    property real cellHeight: 350
    // The poster card's inner width, so the placeholder lines align with the
    // real card's title width.
    readonly property real cardWidth: cellWidth - 16

    columns: Math.max(1, Math.floor(width / cellWidth))
    spacing: 16

    Repeater {
        model: grid.count

        delegate: Item {
            width: grid.cardWidth
            height: grid.cellHeight

            Column {
                anchors.fill: parent
                spacing: 8

                // Poster block, 2:3.
                Skeleton {
                    width: parent.width
                    height: parent.width * 1.5
                    radius: 12
                }

                // Two title lines.
                Skeleton { width: parent.width * 0.85; height: 12; radius: 4 }
                Skeleton { width: parent.width * 0.5; height: 12; radius: 4 }
            }
        }
    }
}