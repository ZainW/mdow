import { useMemo } from 'react'
import { Cpu } from 'lucide-react'
import type {
  CompanionModelOption,
  CompanionModelProvider,
  CompanionModelState,
} from '../../../../shared/types'
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

const GROUPS: Array<{ provider: CompanionModelProvider; label: string }> = [
  { provider: 'openai', label: 'ChatGPT subscription' },
  { provider: 'opencode', label: 'OpenCode Zen' },
  { provider: 'opencode-go', label: 'OpenCode Go' },
]

function nameForValue(options: CompanionModelOption[], value: string | null): string {
  return options.find((option) => option.value === value)?.name ?? 'Choose model'
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
  const unavailable = disabled || state.stale || state.options.length === 0

  return (
    <Combobox
      items={values}
      value={state.currentValue}
      onValueChange={(value) => {
        if (typeof value === 'string' && value !== state.currentValue) onValueChange(value)
      }}
      itemToStringValue={(value) => byValue.get(value)?.name ?? value}
      disabled={unavailable}
    >
      <ComboboxTrigger
        aria-label="Model"
        render={
          <Button
            variant="ghost"
            size="sm"
            className="max-w-56 justify-start gap-1.5 px-2 text-xs font-normal text-muted-foreground"
            title={unavailable ? state.unavailableReason : undefined}
          />
        }
      >
        <Cpu className="size-3.5" aria-hidden />
        <span className="truncate">{nameForValue(state.options, state.currentValue)}</span>
      </ComboboxTrigger>
      <ComboboxContent className="w-80">
        <ComboboxInput aria-label="Search models" placeholder="Search live models…" />
        <ComboboxEmpty>No matching live models.</ComboboxEmpty>
        <ComboboxList>
          {GROUPS.map((group) => {
            const options = state.options.filter((option) => option.provider === group.provider)
            if (options.length === 0) return null
            return (
              <ComboboxGroup key={group.provider}>
                <ComboboxLabel>{group.label}</ComboboxLabel>
                {options.map((option) => (
                  <ComboboxItem key={option.value} value={option.value}>
                    <span className="min-w-0 flex-1">
                      <span className="block truncate">{option.name}</span>
                      {option.description && (
                        <span className="block truncate text-[11px] text-muted-foreground">
                          {option.description}
                        </span>
                      )}
                    </span>
                  </ComboboxItem>
                ))}
              </ComboboxGroup>
            )
          })}
        </ComboboxList>
      </ComboboxContent>
    </Combobox>
  )
}
