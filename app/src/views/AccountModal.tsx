import { useEffect, useMemo, useRef, useState } from "react";
import type { Account, AccountInput, FacetColor, LauncherIcon } from "../lib/api";
import { errorText } from "../lib/api";
import { defaultDirs, firstUnusedColor, slugify } from "../lib/accounts";
import { tildify } from "../lib/format";
import { useApi } from "../lib/useFacet";
import { Avatar, Button, Callout, Checkbox, Field, Modal, Segmented, Stepper, accountStyle, describedBy, rovingKeys } from "../components/ui";
import { useToast } from "../components/Toast";

const STEPS = ["Name", "Folders", "Review"];
const COLORS: FacetColor[] = [1, 2, 3, 4, 5, 6];
const COLOR_NAMES = ["Teal", "Violet", "Rose", "Amber", "Blue", "Green"];
/** Mirrors `profile::RESERVED_IDS` in the core. */
const RESERVED = ["desktop", "code", "auto", "cli", "app", "facet"];
const MAX_NOTES = 2000;
const MAX_PICTURE = 2 * 1024 * 1024;

/** Mirrors `profile::validate_badge`: up to three characters or one emoji. */
export function badgeError(b: string): string | null {
  const t = b.trim();
  if (!t) return null;
  const visible = [...t].filter((c) => !/[‍︎️\u{1f3fb}-\u{1f3ff}\u{e0020}-\u{e007f}]/u.test(c));
  const emoji = [...t].some((c) => c.codePointAt(0)! >= 0x2190);
  if (/\p{Cc}/u.test(t) || visible.length > (emoji ? 4 : 3)) return "Use up to three letters or one emoji.";
  return null;
}

export interface AccountPreset {
  name: string;
  desktopDir: string | null;
  codeDir: string | null;
}

/** Add (three steps) or edit (one page, every option) an account. Writes
 *  only Facet's own files; Claude creates its folders on first start. */
export function AccountModal({ existing, home, account, preset, onClose, onSaved, onDeleted }: { existing: Account[]; home: string; account?: Account; preset?: AccountPreset; onClose: () => void; onSaved: (a: Account) => void; onDeleted?: () => void }) {
  const api = useApi();
  const toast = useToast();
  const editing = Boolean(account);
  const [step, setStep] = useState(editing ? 2 : 0);
  const [name, setName] = useState(account?.name ?? preset?.name ?? "");
  const [shortId, setShortId] = useState(account?.id ?? "");
  const [color, setColor] = useState<FacetColor>(account?.color ?? firstUnusedColor(existing));
  const [badge, setBadge] = useState(account?.badge ?? "");
  const [plan, setPlan] = useState(account?.plan ?? "");
  const [notes, setNotes] = useState(account?.notes ?? "");
  const [email, setEmail] = useState(account?.expectedEmail ?? "");
  const [withDesktop, setWithDesktop] = useState(account ? Boolean(account.desktopDir) : preset ? preset.desktopDir !== null : true);
  const [x11, setX11] = useState(account?.x11Identity ?? existing.length > 0);
  const [tint, setTint] = useState(account?.tint ?? true);
  const [desktopDir, setDesktopDir] = useState(account?.desktopDir ?? preset?.desktopDir ?? "");
  const [codeDir, setCodeDir] = useState(account?.codeDir ?? preset?.codeDir ?? "");
  const [launcherIcon, setLauncherIcon] = useState<LauncherIcon>(account?.launcherIcon ?? "avatar");
  const [terminalDir, setTerminalDir] = useState(account?.terminalDir ?? "");
  const [launcherName, setLauncherName] = useState(account?.launcherName ?? "");
  const [image, setImage] = useState<string | null>(account?.imageUrl ?? null);
  const [imageBusy, setImageBusy] = useState(false);
  const [preview, setPreview] = useState<string | null | "loading" | "error">("loading");
  const [pictureError, setPictureError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [removing, setRemoving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [confirmRemove, setConfirmRemove] = useState(false);
  const fileRef = useRef<HTMLInputElement>(null);

  const id = editing ? shortId.trim() : slugify(name);
  const first = existing.length === 0;
  const defs = useMemo(() => defaultDirs(home, id || "account", first), [home, id, first]);
  const others = existing.filter((a) => a.id !== account?.id);
  const colorOwner = (c: FacetColor) => others.find((a) => a.color === c);

  const idError = !id
    ? editing
      ? "Give it a short name: lowercase letters, digits and dashes."
      : name.trim()
        ? "Use at least one Latin letter or digit; Facet uses it in folder and command names."
        : null
    : !/^[a-z0-9][a-z0-9-]{0,31}$/.test(id)
      ? "Use 1–32 lowercase letters, digits or dashes, starting with a letter or digit."
      : RESERVED.includes(id) && id !== account?.id
        ? `"${id}" is reserved because claude-${id} could hide another command. Pick another name.`
        : others.some((a) => a.id === id)
          ? `An account called "${id}" already exists. Pick another name.`
          : null;
  const nameError = !editing ? idError : null;
  const shortIdError = editing ? idError : null;
  const emailError = email.trim() && !/^[^@\s]+@[^@\s]+$/.test(email.trim()) ? "Enter the full email, for example you@company.com." : null;
  const badgeErr = badgeError(badge);
  const notesError = [...notes].length > MAX_NOTES ? `Notes can be up to ${MAX_NOTES} characters.` : null;
  const terminalError = terminalDir.trim() && !terminalDir.trim().startsWith("/") && !terminalDir.trim().startsWith("~") ? "Use a full path, for example ~/work." : null;
  const anyError = Boolean(!name.trim() || idError || emailError || badgeErr || notesError || terminalError);
  const canNext = step === 0 ? Boolean(name.trim()) && !nameError && !badgeErr : step === 1 ? !emailError : true;
  const dirty = editing
    ? name !== account!.name ||
      shortId !== account!.id ||
      color !== account!.color ||
      badge !== (account!.badge ?? "") ||
      plan !== (account!.plan ?? "") ||
      notes !== (account!.notes ?? "") ||
      email !== (account!.expectedEmail ?? "") ||
      withDesktop !== Boolean(account!.desktopDir) ||
      x11 !== account!.x11Identity ||
      tint !== account!.tint ||
      desktopDir !== (account!.desktopDir ?? "") ||
      codeDir !== account!.codeDir ||
      launcherIcon !== account!.launcherIcon ||
      terminalDir !== (account!.terminalDir ?? "") ||
      launcherName !== (account!.launcherName ?? "")
    : name.trim() !== "" || email.trim() !== "" || plan.trim() !== "";

  const expand = (p: string) => (p.trim() === "~" ? home : p.trim().startsWith("~/") ? home + p.trim().slice(1) : p.trim());
  const folderChanged =
    editing && (expand(codeDir || "") !== account!.codeDir || (withDesktop ? expand(desktopDir || "") !== (account!.desktopDir ?? "") : Boolean(account!.desktopDir)));
  const idChanged = editing && id !== account!.id;

  // On a step change, focus the first field of the new step.
  useEffect(() => {
    if (editing) return;
    const t = setTimeout(() => document.querySelector<HTMLElement>(".modal-body input:not(:disabled)")?.focus(), 0);
    return () => clearTimeout(t);
  }, [step, editing]);

  // Live preview of the menu and dock icon while editing.
  useEffect(() => {
    if (!account || !withDesktop) return;
    let stale = false;
    const t = setTimeout(() => {
      void api
        .launcherPreview(account.id, { style: launcherIcon, badge: badgeErr ? undefined : badge, color, name: name.trim() || undefined })
        .then((url) => !stale && setPreview(url), () => !stale && setPreview("error"));
    }, 150);
    return () => {
      stale = true;
      clearTimeout(t);
    };
  }, [api, account, launcherIcon, badge, badgeErr, color, name, withDesktop, image]);

  const input: AccountInput = {
    id: editing ? id : undefined,
    name: name.trim(),
    color,
    plan: plan.trim() || undefined,
    withDesktop,
    desktopDir: withDesktop ? (desktopDir.trim() ? expand(desktopDir) : undefined) : undefined,
    codeDir: codeDir.trim() ? expand(codeDir) : undefined,
    expectedEmail: email.trim() || undefined,
    tint,
    x11Identity: withDesktop ? x11 : false,
    badge: badge.trim() || undefined,
    notes: notes.trim() || undefined,
    launcherIcon,
    terminalDir: terminalDir.trim() ? expand(terminalDir) : undefined,
    launcherName: launcherName.trim() || undefined,
  };
  const finalDesktop = withDesktop ? input.desktopDir || defs.desktopDir : null;
  const finalCode = input.codeDir || defs.codeDir;

  const save = async () => {
    setSaving(true);
    setError(null);
    try {
      if (idChanged) await api.renameAccount(account!.id, id);
      const saved = await api.saveAccount(input);
      toast(editing ? (idChanged ? `Saved ${saved.name}; its command is now claude-${saved.id}.` : `Saved ${saved.name}.`) : `Added ${saved.name}; launch it to sign in.`);
      onSaved(saved);
    } catch (e) {
      setError(errorText(e));
    } finally {
      setSaving(false);
    }
  };

  const remove = async () => {
    if (!account) return;
    setRemoving(true);
    try {
      await api.deleteAccount(account.id);
      toast(`Facet forgot ${account.name}; its folders and sign-in are unchanged.`);
      onDeleted?.();
    } catch (e) {
      setError(errorText(e));
      setRemoving(false);
    }
  };

  const pickPicture = (file: File | undefined) => {
    if (!file || !account) return;
    setPictureError(null);
    if (file.size > MAX_PICTURE) {
      setPictureError("Pick a picture under 2 MB.");
      return;
    }
    const reader = new FileReader();
    reader.onload = async () => {
      setImageBusy(true);
      try {
        const saved = await api.setAccountImage(account.id, String(reader.result));
        setImage(saved.imageUrl ?? null);
        toast(`Picture set for ${saved.name}.`);
      } catch (e) {
        setPictureError(errorText(e));
      } finally {
        setImageBusy(false);
        if (fileRef.current) fileRef.current.value = "";
      }
    };
    reader.readAsDataURL(file);
  };

  const clearPicture = async () => {
    if (!account) return;
    setImageBusy(true);
    try {
      await api.setAccountImage(account.id, null);
      setImage(null);
      toast(`Removed ${account.name}'s picture.`);
    } catch (e) {
      setPictureError(errorText(e));
    } finally {
      setImageBusy(false);
    }
  };

  const primaryLabel = step < 2 ? "Continue" : editing ? "Save changes" : "Add account";

  const identityFields = (
    <>
      {editing && (
        <div className="picture-row">
          <Avatar name={name || "?"} color={color} badge={badgeErr ? undefined : badge} image={image} size="xl" label={`${name || "Account"} picture`} />
          <div className="stack-sm">
            <div className="actions">
              <Button size="sm" icon="user" loading={imageBusy} onClick={() => fileRef.current?.click()}>
                {image ? "Change picture" : "Choose picture"}
              </Button>
              {image && (
                <Button size="sm" variant="ghost" icon="trash" disabled={imageBusy} onClick={() => void clearPicture()}>
                  Remove picture
                </Button>
              )}
            </div>
            <div className="hint">PNG, JPEG, WebP, GIF or SVG, up to 2 MB. Shown in Facet and on this account's menu and dock icon. It applies at once.</div>
            {pictureError && (
              <div className="field-error" role="alert">
                {pictureError}
              </div>
            )}
            <input ref={fileRef} type="file" accept="image/png,image/jpeg,image/webp,image/gif,image/svg+xml" className="visually-hidden" tabIndex={-1} aria-label="Account picture file" onChange={(e) => pickPicture(e.target.files?.[0])} />
          </div>
        </div>
      )}
      <Field label="Name" htmlFor="acc-name" error={nameError} hint={editing ? "Shown in Facet, the tray, launchers and window titles." : id ? `Short name: ${id}` : "For example Work, Personal or a client's name."}>
        <input id="acc-name" className="input" value={name} onChange={(e) => setName(e.target.value)} aria-invalid={Boolean(nameError)} aria-describedby={describedBy("acc-name", nameError, "hint")} autoComplete="off" />
      </Field>
      {editing && (
        <Field
          label="Short name"
          htmlFor="acc-id"
          error={shortIdError}
          hint={idChanged ? `Facet will rename claude-${account!.id} to claude-${id || "…"} and move the launcher, folder rules and backups. Its Claude folders stay where they are.` : `Used in the claude-${account!.id} command, the launcher and window names.`}
        >
          <input id="acc-id" className="input mono" value={shortId} onChange={(e) => setShortId(e.target.value.toLowerCase())} aria-invalid={Boolean(shortIdError)} aria-describedby={describedBy("acc-id", shortIdError, "hint")} autoComplete="off" spellCheck={false} />
        </Field>
      )}
      <Field label="Badge (optional)" htmlFor="acc-badge" error={badgeErr} hint="Up to three letters or one emoji, shown instead of the first letter. For example W, ACM or 🚀.">
        <input id="acc-badge" className="input" value={badge} onChange={(e) => setBadge(e.target.value)} aria-invalid={Boolean(badgeErr)} aria-describedby={describedBy("acc-badge", badgeErr, "hint")} autoComplete="off" />
      </Field>
      <div className="field">
        <span className="field-label" id="acc-color-l">
          Colour
        </span>
        <div
          className="swatches"
          role="radiogroup"
          aria-labelledby="acc-color-l"
          onKeyDown={(e) => {
            rovingKeys(e, ".swatch", "both");
            const focused = document.activeElement as HTMLElement | null;
            const c = Number(focused?.dataset.color);
            if (c && ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End"].includes(e.key)) setColor(c as FacetColor);
          }}
        >
          {COLORS.map((c, i) => {
            const owner = colorOwner(c);
            return (
              <button
                key={c}
                type="button"
                role="radio"
                data-color={c}
                aria-checked={color === c}
                tabIndex={color === c ? 0 : -1}
                aria-label={owner ? `${COLOR_NAMES[i]}, used by ${owner.name}` : COLOR_NAMES[i]}
                title={owner ? `${COLOR_NAMES[i]} · used by ${owner.name}` : COLOR_NAMES[i]}
                className="swatch"
                style={accountStyle(c)}
                onClick={() => setColor(c)}
              >
                <span aria-hidden="true">{color === c ? "✓" : owner ? [...owner.name][0]?.toUpperCase() : ""}</span>
              </button>
            );
          })}
        </div>
        {colorOwner(color) && <div className="hint">{colorOwner(color)!.name} already uses this colour, so the two will look alike.</div>}
      </div>
      <Field label="Plan (optional)" htmlFor="acc-plan" hint="Only for your own reference, for example Pro, Max or Team.">
        <input id="acc-plan" className="input" value={plan} onChange={(e) => setPlan(e.target.value)} aria-describedby="acc-plan-hint" autoComplete="off" />
      </Field>
      {editing && (
        <Field label="Notes (optional)" htmlFor="acc-notes" error={notesError} hint="For you only: client, billing, what this account is for. Stored in Facet's accounts file.">
          <textarea id="acc-notes" className="input textarea" rows={3} value={notes} onChange={(e) => setNotes(e.target.value)} aria-invalid={Boolean(notesError)} aria-describedby={describedBy("acc-notes", notesError, "hint")} />
        </Field>
      )}
    </>
  );

  const folderHint = (def: string) => (editing ? "Facet does not move files. Point it at the folder you want; Claude creates it on first start. Quit the Desktop app and close Code sessions first." : `Leave empty for ${tildify(def, home)}.`);

  const folderFields = (
    <>
      <Checkbox checked={withDesktop} onChange={setWithDesktop} label="Use the Claude desktop app with this account" hint="Leave unticked for Claude Code in the terminal only." />
      {withDesktop && (
        <Field label="Desktop folder" htmlFor="acc-ddir" hint={folderHint(defs.desktopDir)}>
          <input id="acc-ddir" className="input mono" value={editing ? tildify(desktopDir, home) : desktopDir} placeholder={tildify(defs.desktopDir, home)} onChange={(e) => setDesktopDir(e.target.value)} aria-describedby="acc-ddir-hint" spellCheck={false} />
        </Field>
      )}
      <Field label="Claude Code folder" htmlFor="acc-cdir" hint={folderHint(defs.codeDir)}>
        <input id="acc-cdir" className="input mono" value={editing ? tildify(codeDir, home) : codeDir} placeholder={tildify(defs.codeDir, home)} onChange={(e) => setCodeDir(e.target.value)} aria-describedby="acc-cdir-hint" spellCheck={false} />
      </Field>
      {folderChanged && (
        <Callout tone="warn" icon="alert">
          <p>You are pointing this account at a different folder. Its sign-in, history, settings and skills are whatever that folder holds; the old folder stays as it is.</p>
        </Callout>
      )}
      <Field label="Email you sign in with (optional)" htmlFor="acc-email" error={emailError} hint="Facet compares it with the official sign-in status, so a sign-in in the wrong window is caught.">
        <input id="acc-email" className="input ltr" type="email" value={email} onChange={(e) => setEmail(e.target.value)} aria-invalid={Boolean(emailError)} aria-describedby={describedBy("acc-email", emailError, "hint")} autoComplete="off" />
      </Field>
    </>
  );

  const lookFields = (
    <>
      {withDesktop && <Checkbox checked={x11} onChange={setX11} label="Own dock icon" hint="Runs this Desktop window under X11 so the dock shows it separately with the icon below. Text may look slightly blurry at 125% or 150% display scaling." />}
      {withDesktop && editing && (
        <Field label="Name in the dock and menu" htmlFor="acc-lname" hint={`Leave empty for "${name.trim() || "Account"} · Facet". For example Claude Work.`}>
          <input id="acc-lname" className="input" value={launcherName} placeholder={`${name.trim() || "Account"} · Facet`} onChange={(e) => setLauncherName(e.target.value)} aria-describedby="acc-lname-hint" maxLength={64} autoComplete="off" />
        </Field>
      )}
      {withDesktop && editing && (
        <div className="field">
          <span className="field-label" aria-hidden="true">
            Menu and dock icon
          </span>
          <div className="launcher-row">
            {preview === "loading" ? (
              <span className="launcher-preview launcher-preview--state" aria-label="Loading the icon preview" role="img">
                <span className="spinner" aria-hidden="true" />
              </span>
            ) : preview === "error" || !preview ? (
              <span className="launcher-preview launcher-preview--state hint">Preview unavailable</span>
            ) : (
              <img className="launcher-preview" src={preview} alt={`Preview of ${name || "the account"}'s menu and dock icon`} />
            )}
            <div className="stack-sm">
              <Segmented<LauncherIcon>
                label="Menu and dock icon"
                value={launcherIcon}
                onChange={setLauncherIcon}
                options={[
                  { value: "avatar", label: "Account square" },
                  { value: "claude", label: "Claude app icon" },
                  { value: "claude-badge", label: "Claude icon with badge" },
                ]}
              />
              <div className="hint">
                {launcherIcon === "claude-badge"
                  ? "Uses the Claude app icon already installed on this computer, with this account's badge in the corner. Built here for your own dock only; Facet ships no Claude artwork and is not affiliated with Anthropic."
                  : launcherIcon === "claude"
                    ? "Uses the Claude app's own icon as installed on this computer, unchanged. The dock name tells your accounts apart."
                    : "Facet's square in the account colour, with the picture, badge or first letter."}{" "}
                Saving updates an installed launcher.
              </div>
            </div>
          </div>
        </div>
      )}
      <Checkbox checked={tint} onChange={setTint} label="Tint Facet in this colour" hint="A thin ring around Facet shows which account you are in." />
      {editing && (
        <Field label="Terminal starts in (optional)" htmlFor="acc-tdir" error={terminalError} hint="Folder that Open terminal starts in. Leave empty for your home folder.">
          <input id="acc-tdir" className="input mono" value={terminalDir} placeholder="~/work" onChange={(e) => setTerminalDir(e.target.value)} aria-invalid={Boolean(terminalError)} aria-describedby={describedBy("acc-tdir", terminalError, "hint")} spellCheck={false} />
        </Field>
      )}
    </>
  );

  return (
    <Modal
      title={editing ? `Edit ${account!.name}` : "Add account"}
      onClose={onClose}
      dismissable={!dirty && !saving}
      identity={<Avatar name={name || "?"} color={color} badge={badgeErr ? undefined : badge} image={image} size="lg" />}
      header={!editing && <Stepper steps={STEPS} current={step} />}
      footer={
        <>
          <div className="actions">
            {editing && !confirmRemove && (
              <Button variant="ghost" icon="trash" onClick={() => setConfirmRemove(true)}>
                Remove from Facet
              </Button>
            )}
            {!editing && step > 0 && (
              <Button variant="ghost" onClick={() => setStep(step - 1)}>
                Back
              </Button>
            )}
          </div>
          <div className="actions">
            <Button onClick={onClose}>Cancel</Button>
            <Button key="primary" variant="primary" loading={saving} disabled={step < 2 ? !canNext : anyError} onClick={() => (step < 2 ? setStep(step + 1) : void save())}>
              {primaryLabel}
            </Button>
          </div>
        </>
      }
    >
      {confirmRemove && (
        <Callout
          tone="warn"
          icon="alert"
          action={
            <div className="actions">
              <Button size="sm" autoFocus onClick={() => setConfirmRemove(false)}>
                Keep
              </Button>
              <Button size="sm" variant="danger" loading={removing} onClick={() => void remove()}>
                Remove
              </Button>
            </div>
          }
        >
          <p>Facet forgets this account. Its folders, sign-in and history stay exactly as they are.</p>
        </Callout>
      )}

      {!editing && step === 0 && (
        <>
          <p className="muted">Add only accounts that are yours. Facet keeps your own accounts apart; it is not for sharing accounts.</p>
          {identityFields}
        </>
      )}
      {!editing && step === 1 && (
        <>
          {folderFields}
          {withDesktop && <Checkbox checked={x11} onChange={setX11} label="Own dock icon" hint="Runs this Desktop window under X11 so the dock shows it separately. Text may look slightly blurry at 125% or 150% display scaling." />}
          <Checkbox checked={tint} onChange={setTint} label="Tint Facet in this colour" hint="A thin ring around Facet shows which account you are in." />
        </>
      )}

      {editing && (
        <>
          <section className="modal-section" aria-labelledby="acc-s1">
            <h3 className="section-title" id="acc-s1">
              Name and look
            </h3>
            {identityFields}
          </section>
          <section className="modal-section" aria-labelledby="acc-s2">
            <h3 className="section-title" id="acc-s2">
              Folders and sign-in
            </h3>
            {folderFields}
          </section>
          <section className="modal-section" aria-labelledby="acc-s3">
            <h3 className="section-title" id="acc-s3">
              Launching
            </h3>
            {lookFields}
          </section>
        </>
      )}

      {step === 2 && !editing && (
        <>
          <p>Facet will save this account in its own accounts file. It does not create, move or read any Claude folder; Claude creates them when the account first starts.</p>
          <pre className="code-preview">
            {[`name        ${name.trim()}`, `id          ${id}`, `colour      ${COLOR_NAMES[color - 1]}`, badge.trim() && `badge       ${badge.trim()}`, finalDesktop ? `desktop     ${tildify(finalDesktop, home)}` : "desktop     (none, terminal only)", `claude code ${tildify(finalCode, home)}`, email.trim() && `email       ${email.trim()}`].filter(Boolean).join("\n")}
          </pre>
          <Callout icon="info">
            <p>After adding, Edit sets a picture, notes, the dock icon and where terminals start. To sign in, launch the account and sign in inside its Claude window. If a sign-in link opens in another account's window, close the other Claude windows first.</p>
          </Callout>
        </>
      )}

      {error && (
        <Callout tone="bad" icon="alert">
          <p>{error}</p>
        </Callout>
      )}
    </Modal>
  );
}
