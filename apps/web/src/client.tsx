import { hydrateStart, StartClient } from '@tanstack/react-start/client'
import { createRouter } from './router'

const router = createRouter()

hydrateStart(<StartClient router={router} />)
