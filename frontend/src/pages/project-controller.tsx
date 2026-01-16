import { useState, useEffect, useCallback, useRef } from 'react';
import { useParams } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Loader } from '@/components/ui/loader';
import {
  Bot,
  Send,
  Settings,
  Activity,
  Sparkles,
  User,
  MessageSquarePlus,
  History,
  Trash2,
  CheckCircle2,
  Circle,
  PlayCircle,
  AlertCircle,
} from 'lucide-react';
import {
  projectsApi,
  projectControllersApi,
  tasksApi,
  type ProjectControllerConfig,
  type ProjectControllerConversation,
} from '@/lib/api';
import { cn } from '@/lib/utils';
import { ControllerSettingsDialog } from '@/components/dialogs/controller-settings-dialog';
import type { Project, TaskWithAttemptStatus } from 'shared/types';

interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
}

export function ProjectControllerPage() {
  const { projectId } = useParams<{ projectId: string }>();
  const queryClient = useQueryClient();
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState('');
  const [conversationId, setConversationId] = useState<string | undefined>();
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [showHistory, setShowHistory] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Fetch project data
  const { data: project, isLoading: projectLoading } = useQuery<Project>({
    queryKey: ['project', projectId],
    queryFn: () => projectsApi.getById(projectId!),
    enabled: !!projectId,
  });

  // Fetch controller config
  const { data: controllerConfig } = useQuery<ProjectControllerConfig>({
    queryKey: ['project-controller-config', projectId],
    queryFn: () => projectControllersApi.getConfig(projectId!),
    enabled: !!projectId,
  });

  // Fetch conversation list
  const { data: conversations = [] } = useQuery<ProjectControllerConversation[]>({
    queryKey: ['project-controller-conversations', projectId],
    queryFn: () => projectControllersApi.getConversations(projectId!),
    enabled: !!projectId,
  });

  // Fetch conversation messages when a conversation is selected
  const { data: conversationData, isLoading: messagesLoading } = useQuery({
    queryKey: ['project-controller-conversation', projectId, conversationId],
    queryFn: () => projectControllersApi.getConversation(projectId!, conversationId!),
    enabled: !!projectId && !!conversationId,
  });

  // Load messages from conversation data
  useEffect(() => {
    if (conversationData?.messages) {
      const loadedMessages: ChatMessage[] = conversationData.messages.map((msg) => ({
        id: msg.id,
        role: msg.role as 'user' | 'assistant',
        content: msg.content,
        timestamp: new Date(msg.created_at),
      }));
      setMessages(loadedMessages);
    }
  }, [conversationData]);

  // Fetch recent tasks for activity feed
  const { data: tasks = [] } = useQuery<TaskWithAttemptStatus[]>({
    queryKey: ['project-tasks', projectId],
    queryFn: () => tasksApi.getAll(projectId!),
    enabled: !!projectId,
    refetchInterval: 30000, // Refresh every 30 seconds
  });

  // Get recent tasks sorted by updated_at
  const recentTasks = tasks
    .sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())
    .slice(0, 5);

  // Delete conversation mutation
  const deleteConversationMutation = useMutation({
    mutationFn: (convId: string) => projectControllersApi.deleteConversation(projectId!, convId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['project-controller-conversations', projectId] });
      if (conversationId) {
        startNewConversation();
      }
    },
  });

  // Send message mutation
  const sendMessageMutation = useMutation({
    mutationFn: async (content: string) => {
      return projectControllersApi.sendMessage(projectId!, content, conversationId);
    },
    onSuccess: (response) => {
      // Update conversation ID for future messages
      setConversationId(response.conversation_id);

      // Add assistant message
      const assistantMessage: ChatMessage = {
        id: response.message.id,
        role: 'assistant',
        content: response.message.content,
        timestamp: new Date(response.message.created_at),
      };
      setMessages((prev) => [...prev, assistantMessage]);

      // Refresh conversations list
      queryClient.invalidateQueries({ queryKey: ['project-controller-conversations', projectId] });
    },
    onError: (error) => {
      console.error('Failed to send message:', error);
      // Add error message
      const errorMessage: ChatMessage = {
        id: `error-${Date.now()}`,
        role: 'assistant',
        content: 'Sorry, I encountered an error processing your message. Please try again.',
        timestamp: new Date(),
      };
      setMessages((prev) => [...prev, errorMessage]);
    },
  });

  // Start a new conversation
  const startNewConversation = useCallback(() => {
    setConversationId(undefined);
    setMessages([]);
    setShowHistory(false);
  }, []);

  // Load an existing conversation
  const loadConversation = useCallback((convId: string) => {
    setConversationId(convId);
    setShowHistory(false);
  }, []);

  // Auto-scroll to bottom when messages change
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Add welcome message on mount (only for new conversations)
  useEffect(() => {
    if (project && controllerConfig && messages.length === 0 && !conversationId && !messagesLoading) {
      const welcomeMessage: ChatMessage = {
        id: 'welcome',
        role: 'assistant',
        content: `Hello! I'm ${controllerConfig.name}, the controller for ${project.name}. I can help you manage tasks, coordinate with agents, and answer questions about this project. How can I assist you today?`,
        timestamp: new Date(),
      };
      setMessages([welcomeMessage]);
    }
  }, [project, controllerConfig, messages.length, conversationId, messagesLoading]);

  const sendMessage = useCallback(async () => {
    if (!input.trim() || sendMessageMutation.isPending) return;

    const userMessage: ChatMessage = {
      id: `user-${Date.now()}`,
      role: 'user',
      content: input.trim(),
      timestamp: new Date(),
    };

    setMessages((prev) => [...prev, userMessage]);
    const messageContent = input.trim();
    setInput('');

    sendMessageMutation.mutate(messageContent);
  }, [input, sendMessageMutation]);

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  };

  if (projectLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <Loader message="Loading project..." size={32} />
      </div>
    );
  }

  const displayName = controllerConfig?.name || 'Controller';
  const displayPersonality = controllerConfig?.personality || 'professional';

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b bg-background">
        <div className="flex items-center gap-3">
          <div className="rounded-lg bg-purple-100 p-2 dark:bg-purple-950">
            <Bot className="h-5 w-5 text-purple-600" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h1 className="text-xl font-semibold">{displayName}</h1>
              <Badge variant="secondary" className="text-xs">
                {project?.name}
              </Badge>
            </div>
            <p className="text-sm text-muted-foreground">
              Project Controller - {displayPersonality} mode
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <div className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
            <span>Online</span>
          </div>
          <Button
            variant="ghost"
            size="icon"
            onClick={startNewConversation}
            title="New conversation"
          >
            <MessageSquarePlus className="h-4 w-4" />
          </Button>
          <Button
            variant={showHistory ? 'secondary' : 'ghost'}
            size="icon"
            onClick={() => setShowHistory(!showHistory)}
            title="Conversation history"
          >
            <History className="h-4 w-4" />
          </Button>
          <Button variant="ghost" size="icon" onClick={() => setSettingsOpen(true)}>
            <Settings className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* Conversation History Panel */}
      {showHistory && (
        <div className="border-b bg-muted/50 p-4">
          <div className="max-w-3xl mx-auto">
            <div className="flex items-center justify-between mb-3">
              <h3 className="text-sm font-medium flex items-center gap-2">
                <History className="h-4 w-4" />
                Conversation History
              </h3>
              <Button variant="outline" size="sm" onClick={startNewConversation}>
                <MessageSquarePlus className="h-4 w-4 mr-2" />
                New Conversation
              </Button>
            </div>
            {conversations.length === 0 ? (
              <p className="text-sm text-muted-foreground">No previous conversations</p>
            ) : (
              <div className="grid gap-2 max-h-40 overflow-y-auto">
                {conversations.map((conv) => (
                  <div
                    key={conv.id}
                    className={cn(
                      'flex items-center justify-between p-2 rounded-lg border cursor-pointer hover:bg-muted transition-colors',
                      conversationId === conv.id && 'bg-muted border-primary'
                    )}
                    onClick={() => loadConversation(conv.id)}
                  >
                    <div className="flex-1 min-w-0">
                      <p className="text-sm font-medium truncate">
                        {conv.title || `Conversation ${conv.id.slice(0, 8)}...`}
                      </p>
                      <p className="text-xs text-muted-foreground">
                        {new Date(conv.updated_at).toLocaleDateString()}
                      </p>
                    </div>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-8 w-8 shrink-0"
                      onClick={(e) => {
                        e.stopPropagation();
                        deleteConversationMutation.mutate(conv.id);
                      }}
                    >
                      <Trash2 className="h-4 w-4 text-muted-foreground hover:text-destructive" />
                    </Button>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}

      {/* Main content */}
      <div className="flex-1 flex overflow-hidden">
        {/* Chat area */}
        <div className="flex-1 flex flex-col">
          <ScrollArea className="flex-1 p-4">
            <div className="space-y-4 max-w-3xl mx-auto">
              {messagesLoading && (
                <div className="flex items-center justify-center py-8">
                  <Loader message="Loading conversation..." size={24} />
                </div>
              )}
              {messages.map((message) => (
                <div
                  key={message.id}
                  className={cn(
                    'flex gap-3',
                    message.role === 'user' ? 'justify-end' : 'justify-start'
                  )}
                >
                  {message.role === 'assistant' && (
                    <div className="w-8 h-8 rounded-full bg-purple-100 flex items-center justify-center shrink-0 dark:bg-purple-950">
                      <Bot className="h-4 w-4 text-purple-600" />
                    </div>
                  )}
                  <div
                    className={cn(
                      'rounded-lg px-4 py-2 max-w-[80%]',
                      message.role === 'user'
                        ? 'bg-primary text-primary-foreground'
                        : 'bg-muted'
                    )}
                  >
                    <p className="text-sm whitespace-pre-wrap">{message.content}</p>
                    <p className="text-xs opacity-50 mt-1">
                      {message.timestamp.toLocaleTimeString()}
                    </p>
                  </div>
                  {message.role === 'user' && (
                    <div className="w-8 h-8 rounded-full bg-muted flex items-center justify-center shrink-0">
                      <User className="h-4 w-4" />
                    </div>
                  )}
                </div>
              ))}
              {sendMessageMutation.isPending && (
                <div className="flex gap-3">
                  <div className="w-8 h-8 rounded-full bg-purple-100 flex items-center justify-center shrink-0 dark:bg-purple-950">
                    <Bot className="h-4 w-4 text-purple-600" />
                  </div>
                  <div className="bg-muted rounded-lg px-4 py-2">
                    <div className="flex items-center gap-2">
                      <Sparkles className="h-4 w-4 animate-pulse text-purple-500" />
                      <span className="text-sm text-muted-foreground">Thinking...</span>
                    </div>
                  </div>
                </div>
              )}
              <div ref={messagesEndRef} />
            </div>
          </ScrollArea>

          {/* Input area */}
          <div className="border-t p-4">
            <div className="max-w-3xl mx-auto flex gap-2">
              <Input
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyPress={handleKeyPress}
                placeholder={`Message ${displayName}...`}
                disabled={sendMessageMutation.isPending}
                className="flex-1"
              />
              <Button onClick={sendMessage} disabled={!input.trim() || sendMessageMutation.isPending}>
                <Send className="h-4 w-4" />
              </Button>
            </div>
          </div>
        </div>

        {/* Side panel - Activity */}
        <div className="w-80 border-l bg-muted/30 p-4 hidden lg:block">
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm flex items-center gap-2">
                <Activity className="h-4 w-4" />
                Recent Tasks
              </CardTitle>
            </CardHeader>
            <CardContent>
              {recentTasks.length === 0 ? (
                <div className="text-muted-foreground text-center py-4">
                  <Activity className="h-8 w-8 mx-auto mb-2 opacity-50" />
                  <p>No tasks yet</p>
                  <p className="text-xs mt-1">Tasks will appear here as they're created</p>
                </div>
              ) : (
                <div className="space-y-3">
                  {recentTasks.map((task) => {
                    const StatusIcon =
                      task.status === 'inreview'
                        ? CheckCircle2
                        : task.status === 'inprogress'
                          ? PlayCircle
                          : task.status === 'cancelled'
                            ? AlertCircle
                            : Circle;
                    const statusColor =
                      task.status === 'inreview'
                        ? 'text-green-500'
                        : task.status === 'inprogress'
                          ? 'text-blue-500'
                          : task.status === 'cancelled'
                            ? 'text-red-500'
                            : 'text-muted-foreground';
                    return (
                      <div
                        key={task.id}
                        className="flex items-start gap-2 p-2 rounded-lg hover:bg-muted transition-colors"
                      >
                        <StatusIcon className={cn('h-4 w-4 mt-0.5 shrink-0', statusColor)} />
                        <div className="flex-1 min-w-0">
                          <p className="text-sm font-medium truncate">{task.title}</p>
                          <p className="text-xs text-muted-foreground">
                            {new Date(task.updated_at).toLocaleString()}
                          </p>
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </CardContent>
          </Card>

          <Card className="mt-4">
            <CardHeader className="pb-2">
              <CardTitle className="text-sm flex items-center gap-2">
                <Bot className="h-4 w-4" />
                Controller Info
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Name</span>
                  <span className="font-medium">{displayName}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Mode</span>
                  <span className="capitalize">{displayPersonality}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Project</span>
                  <span className="truncate ml-2">{project?.name}</span>
                </div>
                {controllerConfig?.model && (
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Model</span>
                    <span className="truncate ml-2">{controllerConfig.model}</span>
                  </div>
                )}
              </div>
            </CardContent>
          </Card>
        </div>
      </div>

      {/* Settings Dialog */}
      <ControllerSettingsDialog
        open={settingsOpen}
        onOpenChange={setSettingsOpen}
        projectId={projectId!}
        config={controllerConfig}
      />
    </div>
  );
}
