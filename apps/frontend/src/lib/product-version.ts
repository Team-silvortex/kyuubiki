import frontendPackage from "../../package.json" with { type: "json" };

export const KYUUBIKI_PRODUCT_CODENAME = "moxi";
export const KYUUBIKI_PRODUCT_VERSION = frontendPackage.version;
export const KYUUBIKI_PRODUCT_VERSION_LABEL = `${KYUUBIKI_PRODUCT_CODENAME} ${KYUUBIKI_PRODUCT_VERSION}`;
