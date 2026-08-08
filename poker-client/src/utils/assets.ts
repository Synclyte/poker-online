// ensures pngs are bundled in production builds - otherwise, the server ignores them
const assetModules = import.meta.glob('../assets/**/*.{png,jpg,jpeg,svg,otf,mp3,wav}', {
    eager: true,
    import: 'default',
});

/**
 * Resolves asset lookups to the url
 */
export function getAssetUrl(path: string): string {
    if (!path) return '';
    const relative = path
        .replace(/^\/src\/assets\//, '')
        .replace(/^src\/assets\//, '')
        .replace(/^\/assets\//, '')
        .replace(/^assets\//, '');

    const key = `../assets/${relative}`;
    const resolved = assetModules[key];
    if (typeof resolved === 'string') {
        return resolved;
    }
    return path;
}
