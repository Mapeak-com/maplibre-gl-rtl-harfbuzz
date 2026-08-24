/**
 * The map the debug page draws.
 *
 * It is built here rather than fetched so that the page works with no network, and so that every
 * label on it is one of the samples: the point is to look at text, not at a basemap.
 */

import {SAMPLES} from './samples.ts';

export type LabelSettings = {
    size: number;
    halo: boolean;
    /** Whether labels follow a line, which lays every glyph out on its own angle. */
    alongLine: boolean;
    /** An extra label to place in the middle of the map, from the inspector's text box. */
    custom: string;
};

export function buildStyle(glyphs: string, settings: LabelSettings): unknown {
    return {
        version: 8,
        glyphs,
        sources: {
            samples: {type: 'geojson', data: points(settings.custom)},
            routes: {type: 'geojson', data: routes(settings.custom)},
        },
        layers: [
            {id: 'background', type: 'background', paint: {'background-color': '#0f1115'}},
            {
                id: 'routes',
                type: 'line',
                source: 'routes',
                paint: {'line-color': '#2b3038', 'line-width': 14},
            },
            {
                id: 'route-labels',
                type: 'symbol',
                source: 'routes',
                layout: {
                    'symbol-placement': settings.alongLine ? 'line' : 'point',
                    'text-field': ['get', 'text'],
                    'text-font': ['Noto Sans Regular'],
                    'text-size': settings.size,
                    'text-max-angle': 60,
                },
                paint: {'text-color': '#9fd0ff', ...haloPaint(settings.halo)},
            },
            {
                id: 'sample-dots',
                type: 'circle',
                source: 'samples',
                paint: {'circle-radius': 3, 'circle-color': '#4b8fd6'},
            },
            {
                id: 'sample-labels',
                type: 'symbol',
                source: 'samples',
                layout: {
                    'text-field': ['get', 'text'],
                    'text-font': ['Noto Sans Regular'],
                    'text-size': settings.size,
                    'text-anchor': 'top',
                    // Left to collide as it normally would: the collision boxes come from the same
                    // metrics the shaped glyphs carry, so this is worth exercising too.
                    'text-offset': [0, 0.6],
                    'text-max-width': 12,
                },
                paint: {'text-color': '#f4f6fa', ...haloPaint(settings.halo)},
            },
        ],
    };
}

function haloPaint(halo: boolean) {
    return halo ? {'text-halo-color': '#0f1115', 'text-halo-width': 1.4} : {};
}

function points(custom: string) {
    const features = SAMPLES.map((sample) => ({
        type: 'Feature',
        geometry: {type: 'Point', coordinates: sample.at},
        properties: {text: sample.text, language: sample.language},
    }));

    if (custom.trim()) {
        features.push({
            type: 'Feature',
            geometry: {type: 'Point', coordinates: [0, 0]},
            properties: {text: custom, language: 'from the text box'},
        });
    }

    return {type: 'FeatureCollection', features};
}

/**
 * A line for each script, so that labels can be put along one. Text placed along a line is laid out
 * glyph by glyph at its own angle, which is where a mark that is merely *near* its letter rather
 * than attached to it stops being subtle.
 */
function routes(custom: string) {
    const along = SAMPLES.slice(0, 6).map((sample, index) => ({
        type: 'Feature',
        geometry: {
            type: 'LineString',
            coordinates: [
                [-160, 60 - index * 14],
                [-60, 66 - index * 14],
                [40, 60 - index * 14],
            ],
        },
        properties: {text: custom.trim() || sample.text},
    }));
    return {type: 'FeatureCollection', features: along};
}
