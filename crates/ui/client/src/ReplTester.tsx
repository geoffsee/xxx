import React, { useEffect, useMemo, useState } from "react";
import {
    Box,
    Button,
    Flex,
    Heading,
    Input,
    Text,
    Textarea,
    VStack,
    HStack,
    Spacer,
    Select,
} from "@chakra-ui/react";

type ExecuteResponse = {
    result: string;
    success: boolean;
};

export function ReplTester() {
    const [languages, setLanguages] = useState<string[]>([]);
    const [loadingLangs, setLoadingLangs] = useState(false);
    const [selectedLang, setSelectedLang] = useState<string>("");
    const [code, setCode] = useState<string>("");
    const [running, setRunning] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [response, setResponse] = useState<ExecuteResponse | null>(null);
    const [rawResponse, setRawResponse] = useState<string>("");
    const [upstream, setUpstream] = useState<string>(() =>
        localStorage.getItem("repl_upstream") || "http://localhost:3002"
    );

    const canRun = useMemo(() => !!selectedLang && code.trim().length > 0, [
        selectedLang,
        code,
    ]);

    useEffect(() => {
        const load = async () => {
            setLoadingLangs(true);
            setError(null);
            try {
                const url = new URL("/api/repl/languages", location.origin);
                if (upstream) url.searchParams.set("upstream", upstream);
                const res = await fetch(url);
                if (!res.ok) throw new Error(`Failed to load languages: ${res.status}`);
                const data = await res.json();
                const langs = (data?.languages ?? []) as string[];
                setLanguages(langs);
                if (langs.length > 0) setSelectedLang((cur) => cur || langs[0]);
            } catch (e) {
                setError(String(e));
            } finally {
                setLoadingLangs(false);
            }
        };
        load();
    }, [upstream]);

    const onRun = async () => {
        if (!canRun) return;
        setRunning(true);
        setError(null);
        setResponse(null);
        setRawResponse("");
        try {
            const url = new URL("/api/repl/execute", location.origin);
            if (upstream) url.searchParams.set("upstream", upstream);
            const res = await fetch(url, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ language: selectedLang, code }),
            });
            const text = await res.text();
            setRawResponse(text);
            const json = JSON.parse(text) as ExecuteResponse;
            setResponse(json);
        } catch (e) {
            setError(String(e));
        } finally {
            setRunning(false);
        }
    };

    const placeholderFor = (lang: string) => {
        switch (lang.toLowerCase()) {
            case "python":
                return "print(2 + 2)";
            case "node":
                return "console.log(2 + 2)";
            case "ruby":
                return "puts 2 + 2";
            case "go":
                return (
                    "package main\nimport \"fmt\"\nfunc main() { fmt.Println(2 + 2) }"
                );
            case "rust":
                return "fn main() { println!(\"{}\", 2 + 2); }";
            default:
                return "// Enter code here";
        }
    };

    return (
        <VStack gap={4} align="stretch">
            <Flex justifyContent="space-between" alignItems="start">
                <Box>
                    <Heading size="lg">REPL API Tester</Heading>
                    <Text fontSize="sm" opacity={0.8} mt={1}>
                        Select a language, write code, then run it via the REPL API.
                    </Text>
                </Box>
                <Button
                    colorScheme="blue"
                    disabled={!canRun || running}
                    onClick={onRun}
                    loading={running}
                >
                    {running ? "Running..." : "Run"}
                </Button>
            </Flex>

            <Flex gap={4} alignItems="center">
                <HStack gap={2} alignItems="center" flex="0 0 auto">
                    <Box h="6rem">
                    <Text fontWeight="bold" fontSize="sm" flexShrink={0}>Language</Text>
                    <Select.Root
                        size="sm"
                        width="150px"
                        disabled={loadingLangs}
                        value={[selectedLang]}
                        onValueChange={(e) => setSelectedLang(e.value[0])}
                        positioning={{ sameWidth: false }}
                    >
                        <Select.Trigger>
                            <Select.ValueText placeholder="Select language" />
                        </Select.Trigger>
                        <Select.Content portalRef={undefined}>
                            {languages.map((l) => (
                                <Select.Item key={l} item={l}>
                                    {l}
                                </Select.Item>
                            ))}
                        </Select.Content>
                    </Select.Root>
                    </Box>
                </HStack>
                <Spacer />
                <HStack gap={2} alignItems="center" flex="0 0 auto">
                    <Text fontWeight="bold" fontSize="sm" flexShrink={0}>Upstream</Text>
                    <Input
                        size="sm"
                        width="300px"
                        placeholder="http://localhost:3002"
                        value={upstream}
                        onChange={(e) => {
                            const v = e.currentTarget.value;
                            setUpstream(v);
                            localStorage.setItem("repl_upstream", v);
                        }}
                    />
                </HStack>
            </Flex>

            <Textarea
                placeholder={selectedLang ? placeholderFor(selectedLang) : "Loading languages..."}
                minH="180px"
                fontFamily="mono"
                value={code}
                onChange={(e) => setCode(e.currentTarget.value)}
                bg="bg"
            />

            {error && (
                <Textarea
                    readOnly
                    borderColor="red.solid"
                    fontFamily="mono"
                    value={`Error:\n${error}`}
                    bg="bg"
                />
            )}

            {response && (
                <VStack gap={0} align="stretch" borderWidth="1px" borderColor="border" rounded="md" overflow="hidden">
                    <Flex
                        bg={response.success ? "green.subtle" : "red.subtle"}
                        px={4}
                        py={2}
                        alignItems="center"
                    >
                        <Text fontWeight="bold" color={response.success ? "green.fg" : "red.fg"}>
                            {response.success ? "Success" : "Failure"}
                        </Text>
                        <Spacer />
                        <Text fontSize="sm" color="fg.muted">via /api/repl/execute</Text>
                    </Flex>
                    <Textarea
                        readOnly
                        minH="120px"
                        fontFamily="mono"
                        value={response.result}
                        borderWidth={0}
                        rounded={0}
                        bg="bg"
                    />
                </VStack>
            )}

            {rawResponse && (
                <Box as="details" borderWidth="1px" borderColor="border" rounded="md" p={2}>
                    <Text as="summary" fontWeight="bold" cursor="pointer" mb={2}>
                        Raw response
                    </Text>
                    <Textarea
                        readOnly
                        minH="100px"
                        fontFamily="mono"
                        value={rawResponse}
                        bg="bg"
                    />
                </Box>
            )}
        </VStack>
    );
}

export default ReplTester;
