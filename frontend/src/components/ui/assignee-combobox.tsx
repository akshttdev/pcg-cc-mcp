import * as React from 'react';
import { Check, ChevronsUpDown, User, Bot, X } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/components/ui/command';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import { usersApi, agentsApi, type UserListItem } from '@/lib/api';
import type { AgentWithParsedFields } from 'shared/types';
import { useQuery } from '@tanstack/react-query';
import { useDebouncedValue } from '@/hooks';

interface UserComboboxProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  disabled?: boolean;
}

export function UserCombobox({
  value,
  onChange,
  placeholder = 'Select user...',
  disabled = false,
}: UserComboboxProps) {
  const [open, setOpen] = React.useState(false);
  const [search, setSearch] = React.useState('');
  const debouncedSearch = useDebouncedValue(search, 300);

  const { data: users = [], isLoading } = useQuery({
    queryKey: ['users-search', debouncedSearch],
    queryFn: () =>
      usersApi.list({
        search: debouncedSearch || undefined,
        is_active: true,
        limit: 20,
      }),
    enabled: open,
    staleTime: 30000,
  });

  const selectedUser = users.find((u) => u.id === value || u.username === value);

  const handleSelect = (user: UserListItem) => {
    onChange(user.id);
    setOpen(false);
    setSearch('');
  };

  const handleClear = (e: React.MouseEvent) => {
    e.stopPropagation();
    onChange('');
  };

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          role="combobox"
          aria-expanded={open}
          className="w-full justify-between font-normal"
          disabled={disabled}
        >
          {value ? (
            <span className="flex items-center gap-2 truncate">
              <User className="h-4 w-4 shrink-0 text-muted-foreground" />
              {selectedUser?.full_name || selectedUser?.username || value}
            </span>
          ) : (
            <span className="text-muted-foreground">{placeholder}</span>
          )}
          <div className="flex items-center gap-1">
            {value && (
              <X
                className="h-4 w-4 shrink-0 opacity-50 hover:opacity-100"
                onClick={handleClear}
              />
            )}
            <ChevronsUpDown className="h-4 w-4 shrink-0 opacity-50" />
          </div>
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-[300px] p-0" align="start">
        <Command shouldFilter={false}>
          <CommandInput
            placeholder="Search users..."
            value={search}
            onValueChange={setSearch}
          />
          <CommandList>
            {isLoading ? (
              <div className="py-6 text-center text-sm text-muted-foreground">
                Loading...
              </div>
            ) : users.length === 0 ? (
              <CommandEmpty>
                {search ? 'No users found.' : 'Start typing to search users...'}
              </CommandEmpty>
            ) : (
              <CommandGroup>
                {users.map((user) => (
                  <CommandItem
                    key={user.id}
                    value={user.id}
                    onSelect={() => handleSelect(user)}
                  >
                    <Check
                      className={cn(
                        'mr-2 h-4 w-4',
                        value === user.id ? 'opacity-100' : 'opacity-0'
                      )}
                    />
                    <User className="mr-2 h-4 w-4 text-muted-foreground" />
                    <div className="flex flex-col">
                      <span>{user.full_name || user.username}</span>
                      <span className="text-xs text-muted-foreground">
                        @{user.username}
                      </span>
                    </div>
                  </CommandItem>
                ))}
              </CommandGroup>
            )}
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}

interface AgentComboboxProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  disabled?: boolean;
}

export function AgentCombobox({
  value,
  onChange,
  placeholder = 'Select agent...',
  disabled = false,
}: AgentComboboxProps) {
  const [open, setOpen] = React.useState(false);
  const [search, setSearch] = React.useState('');

  const { data: agents = [], isLoading } = useQuery({
    queryKey: ['agents-list'],
    queryFn: () => agentsApi.listActive(),
    enabled: open,
    staleTime: 60000,
  });

  // Filter agents based on search
  const filteredAgents = React.useMemo(() => {
    if (!search) return agents;
    const lowerSearch = search.toLowerCase();
    return agents.filter(
      (agent) =>
        agent.short_name.toLowerCase().includes(lowerSearch) ||
        agent.designation?.toLowerCase().includes(lowerSearch)
    );
  }, [agents, search]);

  const selectedAgent = agents.find(
    (a) => a.id === value || a.short_name.toLowerCase() === value.toLowerCase()
  );

  const handleSelect = (agent: AgentWithParsedFields) => {
    onChange(agent.short_name);
    setOpen(false);
    setSearch('');
  };

  const handleClear = (e: React.MouseEvent) => {
    e.stopPropagation();
    onChange('');
  };

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          role="combobox"
          aria-expanded={open}
          className="w-full justify-between font-normal"
          disabled={disabled}
        >
          {value ? (
            <span className="flex items-center gap-2 truncate">
              <Bot className="h-4 w-4 shrink-0 text-muted-foreground" />
              {selectedAgent?.short_name || value}
            </span>
          ) : (
            <span className="text-muted-foreground">{placeholder}</span>
          )}
          <div className="flex items-center gap-1">
            {value && (
              <X
                className="h-4 w-4 shrink-0 opacity-50 hover:opacity-100"
                onClick={handleClear}
              />
            )}
            <ChevronsUpDown className="h-4 w-4 shrink-0 opacity-50" />
          </div>
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-[300px] p-0" align="start">
        <Command shouldFilter={false}>
          <CommandInput
            placeholder="Search agents..."
            value={search}
            onValueChange={setSearch}
          />
          <CommandList>
            {isLoading ? (
              <div className="py-6 text-center text-sm text-muted-foreground">
                Loading...
              </div>
            ) : filteredAgents.length === 0 ? (
              <CommandEmpty>No agents found.</CommandEmpty>
            ) : (
              <CommandGroup>
                {filteredAgents.map((agent) => (
                  <CommandItem
                    key={agent.id}
                    value={agent.id}
                    onSelect={() => handleSelect(agent)}
                  >
                    <Check
                      className={cn(
                        'mr-2 h-4 w-4',
                        value.toLowerCase() === agent.short_name.toLowerCase()
                          ? 'opacity-100'
                          : 'opacity-0'
                      )}
                    />
                    <Bot className="mr-2 h-4 w-4 text-muted-foreground" />
                    <div className="flex flex-col">
                      <span>{agent.short_name}</span>
                      {agent.designation && (
                        <span className="text-xs text-muted-foreground">
                          {agent.designation}
                        </span>
                      )}
                    </div>
                  </CommandItem>
                ))}
              </CommandGroup>
            )}
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}
