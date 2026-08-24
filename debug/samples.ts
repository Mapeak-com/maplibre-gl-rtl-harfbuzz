/**
 * The strings the debug page is built around.
 *
 * Each one is here because it goes wrong in a particular way without shaping, and the note says
 * how -- so that looking at the page is a test rather than an impression.
 */

export type Sample = {
    /** What is written. */
    text: string;
    /** The language, for the reader. */
    language: string;
    /** What MapLibre does with this string when nothing shapes it. */
    without: string;
    /** Roughly where to put it on the map. */
    at: [longitude: number, latitude: number];
};

export const SAMPLES: Sample[] = [
    {
        text: 'שְׁדֵרוֹת רוֹטְשִׁילְד',
        language: 'Hebrew with niqqud',
        without:
            'Every vowel point is laid out as its own letter-width glyph after the letter it belongs ' +
            'under, so the word comes out as letters interleaved with floating dots.',
        at: [34.77, 32.07],
    },
    {
        text: 'בְּרֵאשִׁית בָּרָא',
        language: 'Hebrew with niqqud',
        without: 'The same, and the dagesh inside the letter body ends up beside it instead.',
        at: [35.21, 31.77],
    },
    {
        text: 'רחוב הרצל 12',
        language: 'Hebrew, unpointed',
        without:
            'Renders, but only right to left: this one already worked with the old plugin, and is ' +
            'here to show that it still does, and that the house number stays left to right.',
        at: [34.99, 32.79],
    },
    {
        text: 'नई दिल्ली',
        language: 'Hindi (Devanagari)',
        without:
            'The vowel sign ि is written before the consonant it follows, and the conjunct ल्ली is one ' +
            'glyph. Laid out a codepoint at a time it comes out in the wrong order and unjoined.',
        at: [77.21, 28.61],
    },
    {
        text: 'मुंबई महाराष्ट्र',
        language: 'Marathi (Devanagari)',
        without: 'ष्ट्र is a three-consonant conjunct that no codepoint stands for.',
        at: [72.87, 19.07],
    },
    {
        text: 'ঢাকা বাংলাদেশ',
        language: 'Bengali',
        without: 'The vowel sign ে is written before its consonant, and ক + া joins.',
        at: [90.4, 23.81],
    },
    {
        text: 'சென்னை தமிழ்நாடு',
        language: 'Tamil',
        without: 'ன்னை needs reordering and ligature substitution; the ை moves ahead of its consonant.',
        at: [80.27, 13.08],
    },
    {
        text: 'ភ្នំពេញ',
        language: 'Khmer',
        without:
            'The subscript consonant ្ន stacks under its base and ើ splits around it. Nothing about ' +
            'this survives a codepoint-at-a-time layout.',
        at: [104.92, 11.56],
    },
    {
        text: 'กรุงเทพมหานคร',
        language: 'Thai',
        without: 'The tone marks and vowels stack above and below the consonants rather than following them.',
        at: [100.5, 13.76],
    },
    {
        text: 'القاهرة مصر',
        language: 'Arabic',
        without:
            'Letters take initial, medial and final forms. The old plugin handled this by swapping in ' +
            "Unicode's presentation forms, which is a workaround this does not need.",
        at: [31.24, 30.04],
    },
    {
        text: 'مَدِينَةُ ٱلْقَاهِرَة',
        language: 'Arabic with tashkeel',
        without:
            'Arabic has the same problem as pointed Hebrew, and worse: the vowel marks have to be ' +
            'hung on letters that have already changed shape to join their neighbours.',
        at: [31.24, 30.05],
    },
    {
        text: 'תל אביב Tel Aviv',
        language: 'Hebrew first, Latin last',
        without:
            'Not about shaping at all, but about which way the line runs. The first strong character ' +
            'is Hebrew, so the paragraph is right to left and the Latin words sit at the left end.',
        at: [34.8, 32.11],
    },
    {
        text: 'Tel Aviv תל אביב',
        language: 'Latin first, Hebrew last',
        without:
            'The same words the other way round. Now the paragraph is left to right, so the Latin ' +
            'comes first and the Hebrew sits at the right end -- reversed within itself either way.',
        at: [34.78, 32.05],
    },
    {
        text: 'רחוב 42 בתל אביב, Israel',
        language: 'Hebrew, digits and Latin',
        without:
            'Three levels at once: right-to-left Hebrew, a left-to-right number inside it, and a ' +
            'left-to-right word after it. Each has to be reversed only as far as its own level.',
        at: [34.82, 32.09],
    },
];

/** What the inspector starts with. */
export const DEFAULT_INSPECTION = 'שְׁדֵרוֹת רוֹטְשִׁילְד';
