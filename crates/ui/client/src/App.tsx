import {
    Box,
    Container,
    Heading,
    Separator,
    Text,
    VStack,
} from "@chakra-ui/react";
import { Provider } from "./components/ui/provider";
import { ReplTester } from "./ReplTester";
import { Toaster } from "./components/ui/toaster";

export function App() {
    return (
        <Provider>
            <Box className="dark" bg="black" color="fg" minH="100vh" py={12}>
                <Container maxW="5xl">
                    <VStack spacing={8} align="stretch">
                        <Box>
                            <Heading as="h1" size="2xl" mb={2}>
                                REPL Playground
                            </Heading>
                            <Text fontSize="lg" color="fg.muted">
                                Quickly test your API endpoints and REPL integrations
                            </Text>
                        </Box>

                        <Separator />

                        <Box bg="bg.panel" p={8} rounded="xl" borderWidth="1px" borderColor="border">
                            <ReplTester />
                        </Box>

                        {/*<Box bg="bg.panel" p={8} rounded="xl" borderWidth="1px" borderColor="border">*/}
                        {/*    <APITester />*/}
                        {/*</Box>*/}
                    </VStack>
                </Container>
                <Toaster />
            </Box>
        </Provider>
    );
}

export default App;
