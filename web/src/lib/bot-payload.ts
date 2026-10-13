export interface BotForm {
  name: string
  model: string
  prompt: string
  toolsets: string
}

export function buildBotUpdatePayload(form: BotForm) {
  const toolsets = form.toolsets
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean)
  return {
    name: form.name,
    // An explicit empty value tells the backend to clear the field to its default;
    // null would mean "keep the stored value".
    model: form.model.trim(),
    system_prompt: form.prompt.trim(),
    enabled_toolsets: toolsets,
  }
}
