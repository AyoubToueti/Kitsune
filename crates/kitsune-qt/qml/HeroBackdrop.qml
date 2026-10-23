// A full-bleed hero backdrop that degrades gracefully.
//
// A wide `bannerImage` is used as-is. When a work has none -- common for a new
// show, where NEITHER source has uploaded wide artwork -- the PORTRAIT cover is
// used instead, but only across the CLEAR area.
//
// That last part is the whole point. Filling a 1280px-wide strip with a 550px
// portrait means scaling it ~2.3x and throwing most of it away, which leaves an
// unrecognisable crop of a face. Drawing it only where the text column is not
// covering it needs ~1.5x, keeps the composition readable, and the left edge
// fades into the page so there is no seam.
import QtQuick
import QtQuick.Effects
import com.kitsune.app

Item {
    id: backdrop

    /// The wide artwork, when the provider supplied one.
    property string banner: ""
    /// The portrait cover, used when there is no banner.
    property string cover: ""
    /// Where the clear area starts, as a fraction of the width.
    ///
    /// The hero's text column occupies the left ~36%, so the borrowed cover is
    /// drawn from there to the right edge only. A caller with a different
    /// layout passes its own value.
    property real clearStart: 0.36

    readonly property bool borrowed: banner === "" && cover !== ""
    readonly property int clearX: Math.round(width * clearStart)

    // The sharp image. Only ever a real banner: a portrait is never shown
    // cropped here, because that is the outcome this component exists to avoid.
    //
    // `mipmap` matters because a banner is DOWNSCALED: AniList serves 1900x400
    // and the hero is 1280x360, so the GPU is minifying every pixel. Without
    // mipmaps that sampling aliases, which is what makes fine line art shimmer
    // as the carousel slides. `smooth` is the default but is set explicitly:
    // the nearest-neighbour alternative looks visibly blocky here, and relying
    // on a default for something this visible is not worth the risk.
    Image {
        anchors.fill: parent
        source: backdrop.banner
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        mipmap: true
        smooth: true
        visible: backdrop.banner !== ""
    }

    // The borrowed cover, confined to the clear area. Clipped so the image
    // cannot bleed back under the text it is meant to avoid.
    Item {
        visible: backdrop.borrowed
        x: backdrop.clearX
        width: backdrop.width - backdrop.clearX
        height: backdrop.height
        clip: true

        // The blur source.
        //
        // `mipmap` is deliberately OFF here. This image is only ever consumed
        // by the blur pass, which averages neighbouring texels anyway, so
        // mipmaps would cost memory and buy nothing. `smooth: false` keeps the
        // upload path cheap; the blur is what removes the pixel structure.
        Image {
            id: washSource
            anchors.fill: parent
            source: backdrop.borrowed ? backdrop.cover : ""
            fillMode: Image.PreserveAspectCrop
            asynchronous: true
            mipmap: false
            smooth: false
            visible: false
        }

        MultiEffect {
            anchors.fill: parent
            source: washSource
            blurEnabled: true
            // Light touch. The image is no longer scaled ~2.3x across the whole
            // strip, so it does not need heavy blur to hide an arbitrary crop --
            // and too much blur throws away the artwork entirely, which defeats
            // the point of using it. Just enough to soften edges and stop the
            // frame reading as a cropped photograph.
            blur: 0.55
            blurMax: 32
            // Pull the saturation down a little: a sharp poster next to body
            // text competes with it, a slightly muted one sits behind it.
            saturation: -0.2
        }

        // Blend the left edge into the page. The image starts where the text
        // column ends, and a hard vertical seam there would look like a
        // layout bug rather than artwork.
        Rectangle {
            anchors.fill: parent
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0.0; color: Theme.heroFadeStart }
                GradientStop { position: 0.18; color: Theme.heroFadeMid }
                GradientStop { position: 0.45; color: "transparent" }
            }
        }
    }
}