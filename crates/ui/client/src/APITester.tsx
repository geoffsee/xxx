import React, { useRef, type FormEvent } from "react";
import {
    Box,
    Button,
    Heading,
    Input,
    Text,
    Textarea,
    VStack,
    HStack,
    Select, Spacer, Flex,
} from "@chakra-ui/react";

export function APITester() {
    const responseInputRef = useRef<HTMLTextAreaElement>(null);
    const [method, setMethod] = React.useState("GET");

    const testEndpoint = async (e: FormEvent<HTMLFormElement>) => {
        e.preventDefault();

        try {
            const form = e.currentTarget;
            const formData = new FormData(form);
            const endpoint = formData.get("endpoint") as string;
            const url = new URL(endpoint, location.href);
            const res = await fetch(url, { method });

            const data = await res.json();
            responseInputRef.current!.value = JSON.stringify(data, null, 2);
        } catch (error) {
            responseInputRef.current!.value = String(error);
        }
    };

    return (
        <VStack gap={4} align="stretch">
            <Box>
                <Heading size="lg">Generic API Tester</Heading>
                <Text fontSize="sm" color="fg.muted" mt={1}>
                    Quickly hit any local endpoint.
                </Text>
            </Box>

            <form onSubmit={testEndpoint}>
                <HStack gap={2}>
                    {/*<Select.Root*/}
                    {/*    size="sm"*/}
                    {/*    width="100px"*/}
                    {/*    value={[method]}*/}
                    {/*    onValueChange={(e) => setMethod(e.value[0])}*/}
                    {/*    positioning={{ sameWidth: false }}*/}
                    {/*>*/}
                    {/*    <Select.Trigger>*/}
                    {/*        <Select.ValueText />*/}
                    {/*    </Select.Trigger>*/}
                    {/*    <Select.Content portalRef={undefined}>*/}
                    {/*        <Select.Item item="GET">GET</Select.Item>*/}
                    {/*        <Select.Item item="PUT">PUT</Select.Item>*/}
                    {/*    </Select.Content>*/}
                    {/*</Select.Root>*/}
                    <Flex gap={4} alignItems="center">
                        <HStack gap={2} alignItems="center" flex="0 0 auto">
                            <Text fontWeight="bold" fontSize="sm">Method</Text>
                            <select
                                value={method}
                                onChange={(e) => setMethod(e.currentTarget.value)}
                                style={{
                                    padding: '0.5rem',
                                    borderRadius: '0.375rem',
                                    border: '1px solid rgba(255, 255, 255, 0.16)',
                                    background: 'transparent',
                                    color: 'inherit',
                                    cursor: 'pointer'
                                }}
                            >
                                <option value="GET">GET</option>
                                <option value="POST">POST</option>
                            </select>
                        </HStack>
                        <Spacer />

                    </Flex>
                    <Input
                        type="text"
                        name="endpoint"
                        defaultValue="/api/repl/languages"
                        placeholder="/api/repl/languages"
                        flex={1}
                    />
                    <Button type="submit" colorScheme="blue">
                        Send
                    </Button>
                </HStack>
            </form>
            <Textarea
                ref={responseInputRef}
                readOnly
                placeholder="Response will appear here..."
                minH="200px"
                fontFamily="mono"
                bg="bg"
            />
        </VStack>
    );
}
