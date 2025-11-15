"use client";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Send, Loader2, Brain, Search, Activity } from "lucide-react";
import { useRef, useEffect, useState, FormEvent, ChangeEvent } from "react";
import { Message, MessageContent, MessageResponse } from "@/components/ai-elements/message";
import type { UIMessage } from "ai";

export function ChatInterface() {
  const [messages, setMessages] = useState<UIMessage[]>([]);
  const [input, setInput] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const handleInputChange = (e: ChangeEvent<HTMLInputElement>) => {
    setInput(e.target.value);
  };

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    if (!input.trim() || isLoading) return;

    const userInput = input;
    setInput("");

    // Add user message
    const userMessage: UIMessage = {
      id: Date.now().toString(),
      role: "user",
      parts: [{ type: "text", text: userInput }],
    };
    setMessages((prev) => [...prev, userMessage]);
    setIsLoading(true);

    try {
      const response = await fetch("/api/chat", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ messages: [...messages, userMessage] }),
      });

      if (!response.ok) throw new Error("Failed to get response");
      if (!response.body) throw new Error("No response body");

      const reader = response.body.getReader();
      const decoder = new TextDecoder();

      let assistantMessage: UIMessage = {
        id: (Date.now() + 1).toString(),
        role: "assistant",
        parts: [{ type: "text", text: "" }],
      };
      setMessages((prev) => [...prev, assistantMessage]);

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        const text = decoder.decode(value);
        assistantMessage.parts[0] = {
          type: "text",
          text: (assistantMessage.parts[0] as any).text + text,
        };
        setMessages((prev) =>
          prev.map((m) => (m.id === assistantMessage.id ? { ...assistantMessage } : m))
        );
      }
    } catch (error) {
      console.error("Error:", error);
    } finally {
      setIsLoading(false);
    }
  };

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  return (
    <div className="flex flex-col h-screen bg-background">
      {/* Header */}
      <header className="border-b">
        <div className="container mx-auto px-4 py-4">
          <div className="flex items-center gap-3">
            <Brain className="h-8 w-8 text-primary" />
            <div>
              <h1 className="text-2xl font-bold">EVNA</h1>
              <p className="text-sm text-muted-foreground">
                Context Synthesis & Semantic Search
              </p>
            </div>
          </div>
        </div>
      </header>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto">
        <div className="container mx-auto px-4 py-6 max-w-4xl">
          {messages.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full space-y-6">
              <Brain className="h-24 w-24 text-muted-foreground/30" />
              <div className="text-center space-y-2">
                <h2 className="text-2xl font-semibold">Welcome to EVNA</h2>
                <p className="text-muted-foreground max-w-md">
                  Your AI agent for context synthesis and semantic search. Ask me about your
                  past conversations, perform morning brain boots, or search your history.
                </p>
              </div>
              
              {/* Quick Actions */}
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4 w-full max-w-2xl mt-8">
                <Card>
                  <CardHeader className="pb-3">
                    <Brain className="h-6 w-6 text-primary mb-2" />
                    <CardTitle className="text-base">Brain Boot</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <CardDescription className="text-sm">
                      Morning check-in combining recent activity with semantic search
                    </CardDescription>
                  </CardContent>
                </Card>
                
                <Card>
                  <CardHeader className="pb-3">
                    <Search className="h-6 w-6 text-primary mb-2" />
                    <CardTitle className="text-base">Semantic Search</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <CardDescription className="text-sm">
                      Deep search across conversation history using natural language
                    </CardDescription>
                  </CardContent>
                </Card>
                
                <Card>
                  <CardHeader className="pb-3">
                    <Activity className="h-6 w-6 text-primary mb-2" />
                    <CardTitle className="text-base">Active Context</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <CardDescription className="text-sm">
                      Query recent activity across different clients and projects
                    </CardDescription>
                  </CardContent>
                </Card>
              </div>
            </div>
          ) : (
            <div className="space-y-4">
              {messages.map((message) => (
                <Message key={message.id} from={message.role}>
                  <MessageContent>
                    {message.parts.map((part, index) => {
                      if (part.type === "text") {
                        return <MessageResponse key={index}>{part.text}</MessageResponse>;
                      }
                      // Handle other part types (tool calls, etc.) if needed
                      return null;
                    })}
                  </MessageContent>
                </Message>
              ))}
              {isLoading && (
                <Message from="assistant">
                  <MessageContent>
                    <Loader2 className="h-4 w-4 animate-spin" />
                  </MessageContent>
                </Message>
              )}
              <div ref={messagesEndRef} />
            </div>
          )}
        </div>
      </div>

      {/* Input */}
      <div className="border-t">
        <div className="container mx-auto px-4 py-4 max-w-4xl">
          <form onSubmit={handleSubmit} className="flex gap-2">
            <Input
              value={input}
              onChange={handleInputChange}
              placeholder="Ask EVNA about your conversations, do a brain boot, or search your history..."
              disabled={isLoading}
              className="flex-1"
            />
            <Button type="submit" disabled={isLoading}>
              {isLoading ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Send className="h-4 w-4" />
              )}
            </Button>
          </form>
        </div>
      </div>
    </div>
  );
}
