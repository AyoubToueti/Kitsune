// The detail page's loading placeholder, shown while `getAnime` is in flight
// (mirrors `DetailPageSkeleton.svelte`). It reproduces the real page's regions
// at their real sizes -- the 360px hero (banner backdrop + 190x285 poster + the
// info column), the episode grid, and the sidebar (trailer + related) -- so the
// real content replaces it without a shift.
import QtQuick
import QtQuick.Layouts

Item {
    id: skeleton

    // Whole-page scroll, matching DetailPage's own Flickable so the placeholder
    // scrolls the same way the real page will.
    Flickable {
        anchors.fill: parent
        contentWidth: width
        contentHeight: body.height
        clip: true
        boundsBehavior: Flickable.StopAtBounds

        Column {
            id: body
            width: parent.width
            spacing: 24

            // --- Hero ---------------------------------------------------
            Rectangle {
                width: parent.width
                height: 360
                color: Theme.surface
                clip: true

                // Banner backdrop.
                Skeleton {
                    anchors.fill: parent
                    radius: 0
                    opacity: 0.4
                }

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: Theme.pad
                    spacing: 24

                    // Poster.
                    Skeleton {
                        Layout.preferredWidth: 190
                        Layout.preferredHeight: 285
                        Layout.alignment: Qt.AlignVCenter
                        //radius: Theme.radius
                    }

                    // Info column: title, genre chips, action bar, metadata,
                    // synopsis.
                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.alignment: Qt.AlignVCenter
                        spacing: 12

                        Skeleton {
                            Layout.fillWidth: true
                            Layout.maximumWidth: 420
                            height: 34
                            radius: 8
                        }

                        // Genre chips.
                        Row {
                            spacing: 6
                            Repeater {
                                model: 5
                                delegate: Skeleton { width: 64; height: 28; radius: 8 }
                            }
                        }

                        // Action bar (the play CTA).
                        Skeleton { width: 150; height: 40; radius: 20 }

                        // Metadata strip.
                        Row {
                            spacing: 16
                            Skeleton { width: 56; height: 14; radius: 4 }
                            Skeleton { width: 72; height: 14; radius: 4 }
                            Skeleton { width: 48; height: 14; radius: 4 }
                        }

                        // Synopsis lines.
                        Column {
                            Layout.fillWidth: true
                            spacing: 8
                            Skeleton { width: parent.width; height: 12; radius: 4 }
                            Skeleton { width: parent.width * 0.9; height: 12; radius: 4 }
                            Skeleton { width: parent.width * 0.7; height: 12; radius: 4 }
                        }
                    }
                }
            }

            // --- Episodes + sidebar -------------------------------------
            RowLayout {
                x: Theme.pad
                width: parent.width - Theme.pad * 2
                spacing: 24

                // Episode grid.
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 12

                    Skeleton { 
                        width: 120;
                         height: 20; 
                        //radius: 4 
                    }

                    Grid {
                        Layout.fillWidth: true
                        columns: 4
                        spacing: 12
                        Repeater {
                            model: 8
                            delegate: Skeleton {
                                width: (parent.width - 36) / 4
                                height: 84
                                //radius: Theme.radius
                            }
                        }
                    }
                }

                // Sidebar: trailer tile + three related rows.
                ColumnLayout {
                    Layout.preferredWidth: 300
                    spacing: 16

                    // Trailer tile
                    Skeleton { 
                        Layout.fillWidth: true; 
                        height: 160; 
                        //radius: Theme.radius 
                    }

                    Skeleton { width: 120; height: 18; radius: 4 }

                    Repeater {
                        model: 3
                        delegate: RowLayout {
                            Layout.fillWidth: true
                            spacing: 12
                            Skeleton { 
                                Layout.preferredWidth: 48;
                                Layout.preferredHeight: 68; 
                                //radius: 8 
                            }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 8
                                Skeleton { Layout.fillWidth: true; height: 12; radius: 4 }
                                Skeleton { Layout.preferredWidth: 80; height: 10; radius: 4 }
                            }
                        }
                    }
                }
            }
        }
    }
}