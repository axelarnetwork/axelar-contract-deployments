module.exports = {
    printWidth: 140,
    singleQuote: true,
    tabWidth: 4,
    useTabs: false,
    bracketSpacing: true,
    importOrder: ['^[./]'],
    importOrderSeparation: true,
    importOrderSortSpecifiers: true,
    plugins: ['@trivago/prettier-plugin-sort-imports'],
    endOfLine: 'auto', // maintains existing line endings
    proseWrap: 'preserve', // don't change line wrapping
    overrides: [
        {
            files: '*.js',
            options: {
                trailingComma: 'all',
            },
        },
        {
            files: '*.ts',
            options: {
                trailingComma: 'all',
            },
        },
        {
            files: '*.json',
            options: {
                tabWidth: 2,
                parser: 'json-stringify',
            },
        },
        {
            files: '*.yaml',
            options: {
                tabWidth: 2,
            },
        },
    ],
};
