import { useMemo } from 'react'
import type { CompanionModelOption, CompanionModelState } from '../../../../shared/types'
import { Button } from '../ui/button'
import {
  Combobox,
  ComboboxContent,
  ComboboxEmpty,
  ComboboxGroup,
  ComboboxInput,
  ComboboxItem,
  ComboboxLabel,
  ComboboxList,
  ComboboxTrigger,
} from '../ui/combobox'

function groupByProvider(options: CompanionModelOption[]) {
  const groups = new Map<string, { label: string; options: CompanionModelOption[] }>()
  for (const option of options) {
    const group = groups.get(option.providerId)
    if (group) group.options.push(option)
    else groups.set(option.providerId, { label: option.providerName, options: [option] })
  }
  return [...groups.entries()]
}

export function CompanionModelPicker({
  state,
  disabled = false,
  onValueChange,
}: {
  state: CompanionModelState
  disabled?: boolean
  onValueChange: (value: string) => void
}) {
  const values = useMemo(() => state.options.map((option) => option.value), [state.options])
  const byValue = useMemo(
    () => new Map(state.options.map((option) => [option.value, option])),
    [state.options],
  )
  const groups = useMemo(() => groupByProvider(state.options), [state.options])
  const unavailable = disabled || state.stale || state.options.length === 0
  const current = state.currentValue ? byValue.get(state.currentValue) : undefined
  const label = current?.name ?? (state.stale ? 'Loading models…' : 'Choose a model')

  return (
    <Combobox
      items={values}
      value={state.currentValue}
      onValueChange={(value) => {
        if (typeof value === 'string' && value !== state.currentValue) onValueChange(value)
      }}
      itemToStringValue={(value) => {
        const option = byValue.get(value)
        return option ? `${option.name} ${option.providerName}` : value
      }}
      disabled={unavailable}
    >
      <ComboboxTrigger
        aria-label={`Model: ${label}`}
        render={
          <Button
            variant="ghost"
            size="xs"
            className="max-w-full min-w-0 justify-start gap-1 px-1.5 font-normal text-muted-foreground"
            title={unavailable ? state.unavailableReason : current?.providerName}
          />
        }
      >
        <span className="truncate">{label}</span>
      </ComboboxTrigger>
      <ComboboxContent className="w-80">
        <ComboboxInput aria-label="Search models" placeholder="Search models…" />
        <ComboboxEmpty>No matching models.</ComboboxEmpty>
        <ComboboxList>
          {groups.map(([providerId, group]) => (
            <ComboboxGroup key={providerId}>
              <ComboboxLabel>{group.label}</ComboboxLabel>
              {group.options.map((option) => (
                <ComboboxItem key={option.value} value={option.value}>
                  <span className="min-w-0 flex-1 truncate">{option.name}</span>
                </ComboboxItem>
              ))}
            </ComboboxGroup>
          ))}
        </ComboboxList>
      </ComboboxContent>
    </Combobox>
  )
}
