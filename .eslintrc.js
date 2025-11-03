module.exports = {
  parser: "@typescript-eslint/parser", // Specifies the ESLint parser
  extends: [
    // Uses the recommended rules from the @typescript-eslint/eslint-plugin
    "plugin:@typescript-eslint/recommended",

    // Uses eslint-config-prettier to disable ESLint rules from @typescript-eslint/eslint-plugin
    // that would conflict with prettier
    "prettier",

    // Enables eslint-plugin-prettier and eslint-config-prettier. This will display prettier errors
    // as ESLint errors. Make sure this is always the last configuration in the extends array.
    "plugin:prettier/recommended",
  ],
  parserOptions: {
    ecmaVersion: 2018, // Allows for the parsing of modern ECMAScript features
    sourceType: "module", // Allows for the use of imports
  },
};
